/** Requests keep distinct ids when the page moves to another type or text. */
let next = 0;

function readingKey({ uri, cfg, owner, module }) {
  return JSON.stringify([uri, cfg ?? "", owner, module]);
}

function scope({ cfg, owner, module }) {
  return owner === undefined ? { cfg } : { cfg, owner, module };
}

/** The host's references and module paths for one drawing's type, text and cfg. */
export class TypeContext {
  #files = new Map();
  #locations = new Map();
  #referrers = null;
  #definitions = null;
  #modulesId = 0;
  #places = [];
  #checking = false;
  #refreshing = false;
  #owner;
  version = 0;

  constructor(path, cfg, start, waitForOwner = false) {
    this.path = path;
    this.start = start;
    this.cfg = cfg;
    this.#refreshing = waitForOwner;
  }

  /** The bindings number their sources from the focus, then each other file. */
  sources(uri, text) {
    const files = [...this.#files.values()].filter(
      (file) => file.uri !== uri || file.text === text,
    );
    const local = files.filter((file) => file.uri === uri);
    const cfg = this.cfg ?? local[0]?.cfg ?? "";
    const selected = this.#owner;
    const own =
      local.find((file) => file.focus && file.cfg === cfg) ??
      local.find(
        (file) =>
          file.cfg === cfg &&
          (!file.resolved ||
            !selected ||
            (file.owner === selected.owner && file.module === selected.module)),
      );
    const focus = {
      uri,
      cfg,
      offsets: local.find((file) => !file.resolved)?.offsets ?? [],
      ...own,
      ...selected,
    };
    const variants = files.filter(
      (file) => file !== own && file.offsets.length,
    );
    const facts = (file) => ({
      uri: file.uri,
      offsets: file.offsets,
      resolved: file.resolved,
      owner: file.owner,
      module: file.module,
      repeated: file.repeated,
      ...this.#locations.get(readingKey(file)),
    });
    return {
      uris: [uri, ...variants.map((file) => file.uri)],
      readings: [focus, ...variants].map(scope),
      texts: [text, ...variants.map((file) => file.text)],
      context: {
        ...facts(focus),
        files: variants.map((file) => ({
          text: file.text,
          cfg: file.cfg,
          ...facts(file),
        })),
      },
    };
  }

  /** A refresh keeps its finished diagram until all replacements can be drawn together. */
  canPublish() {
    if (
      this.#refreshing &&
      (this.#checking || this.#definitions || this.#modulesId)
    )
      return false;
    this.#refreshing = false;
    return true;
  }

  /** Refresh even when a shared source is still waiting for its first complete drawing. */
  refresh(host) {
    if (this.#referrers) {
      this.#refreshing = true;
      this.#requestReferences(
        host,
        this.#referrers.uri,
        this.#referrers.offset,
      );
    }
  }

  #requestReferences(host, uri, offset) {
    this.#referrers = { id: ++next, uri, offset };
    this.#definitions = null;
    this.#checking = true;
    host.send({ type: "referrers", ...this.#referrers });
  }

  /** Asks once per set of places, and again when the editor's engine changes. */
  ask(host, cards, uris, refresh = false, readings) {
    if (refresh || !this.#referrers) {
      this.#refreshing ||= refresh;
      this.#requestReferences(host, uris[0], cards[0].start);
      if (this.#refreshing) return;
    }
    const places = cards.map(({ source, path, start: offset }) => {
      const uri = uris[source];
      return {
        uri,
        path,
        offset,
        ...(readings?.[source] ?? { cfg: this.cfg }),
      };
    });
    // The completed drawing can reorder or omit cards after learning their places.
    const asked = new Set(this.#places.map((place) => JSON.stringify(place)));
    if (places.every((place) => asked.has(JSON.stringify(place)))) {
      return;
    }
    this.#places = places;
    this.#modulesId = ++next;
    host.send({
      type: "modules",
      id: this.#modulesId,
      uri: uris[0],
      places,
    });
  }

  /** The reference provider can miss targets behind a renamed re-export. */
  async checkDefinitions(engine, host, text, cfg, current) {
    const focus = this.#referrers;
    const files = this.#files;
    const active = () =>
      focus === this.#referrers && files === this.#files && current();
    const checks = new Map();
    for (const file of files.values()) {
      if (!active()) return false;
      const own = file.uri === focus.uri;
      if (own && file.text !== text) continue;
      const targets = new Set(file.unknown);
      const variants = file.resolved
        ? []
        : new Set(own ? [file.cfg, cfg] : [file.cfg]);
      for (const enabled of variants) {
        if (!active()) return false;
        try {
          if (!file.targets.has(enabled)) {
            file.targets.set(
              enabled,
              engine.relationshipTargets(file.text, enabled),
            );
          }
          for (const offset of await file.targets.get(enabled))
            targets.add(offset);
        } catch {
          // Retry incomplete Rust when another snapshot is available.
          file.targets.delete(enabled);
        }
      }
      const known = new Set(file.offsets);
      for (const offset of targets) {
        if (known.has(offset)) continue;
        const key = JSON.stringify([file.uri, offset]);
        let check = checks.get(key);
        if (!check) {
          check = { place: { uri: file.uri, offset }, files: [] };
          checks.set(key, check);
        }
        check.files.push(file);
      }
    }
    if (!active()) return false;
    this.#checking = false;
    if (!checks.size) return true;
    const requested = [...checks.values()];
    const id = ++next;
    this.#definitions = { id, requested };
    host.send({
      type: "definitions",
      uri: focus.uri,
      offset: focus.offset,
      id,
      places: requested.map((check) => check.place),
    });
  }

  /** Only the latest request of each kind can change the drawing. */
  receive(message) {
    if (message.type === "referrers" && message.id === this.#referrers?.id) {
      this.#owner = message.focus;
      if (this.#owner) this.cfg = this.#owner.cfg;
      const files = new Map();
      for (const reading of message.files) {
        const key = readingKey(reading);
        const previous = this.#files.get(key);
        files.set(key, {
          ...reading,
          offsets: [...reading.offsets],
          targets:
            previous?.text === reading.text ? previous.targets : new Map(),
        });
      }
      this.#files = files;
      this.#checking = true;
      this.#definitions = null;
      // A new search can bring a previously excluded declaration into the workspace.
      this.#locations.clear();
      this.#places = [];
      this.#modulesId = 0;
    } else if (
      message.type === "definitions" &&
      message.id === this.#definitions?.id
    ) {
      const { requested } = this.#definitions;
      this.#definitions = null;
      let found = false;
      requested.forEach(({ place, files }, index) => {
        if (message.matches[index] === true) {
          for (const file of files) file.offsets.push(place.offset);
          found = true;
        }
      });
      if (!found && !this.#refreshing) return false;
    } else if (
      message.type === "modules" &&
      this.#modulesId &&
      message.id === this.#modulesId
    ) {
      this.#modulesId = 0;
      this.#places.forEach((place, index) => {
        const { path } = place;
        const key = readingKey(place);
        const facts = this.#locations.get(key) ?? {
          modules: Object.create(null),
          places: Object.create(null),
        };
        this.#locations.set(key, facts);
        const module = message.modules[index];
        if (typeof module === "string") facts.modules[path] = module;
        else delete facts.modules[path];
        const placements = message.placements?.[index];
        if (Array.isArray(placements)) facts.places[path] = placements;
        else delete facts.places[path];
      });
    } else {
      return false;
    }
    ++this.version;
    return true;
  }
}
