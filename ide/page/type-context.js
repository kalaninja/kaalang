/** Requests keep distinct ids when the page moves to another type or text. */
let next = 0;

/** The host's references and module paths for one drawing's type, text and cfg. */
export class TypeContext {
  #files = new Map();
  #modules = new Map();
  #placements = new Map();
  #referrers = null;
  #definitions = null;
  #modulesId = 0;
  #places = [];
  version = 0;

  constructor(path) {
    this.path = path;
  }

  /** The bindings number their sources from the focus, then each other file. */
  sources(uri, text) {
    const own = this.#files.get(uri);
    const others = [...this.#files.values()].filter(
      (file) => file.uri !== uri && file.offsets.length > 0,
    );
    return {
      uris: [uri, ...others.map((file) => file.uri)],
      texts: [text, ...others.map((file) => file.text)],
      context: {
        offsets: own?.text === text ? own.offsets : [],
        modules: this.#modules.get(uri),
        places: this.#placements.get(uri),
        files: others.map((file) => ({
          text: file.text,
          cfg: file.cfg,
          offsets: file.offsets,
          modules: this.#modules.get(file.uri),
          places: this.#placements.get(file.uri),
        })),
      },
    };
  }

  /** Asks once per set of places, and again when the editor's engine changes. */
  ask(host, cards, uris, refresh = false) {
    if (refresh || !this.#referrers) {
      this.#referrers = { id: ++next, uri: uris[0], offset: cards[0].start };
      this.#definitions = null;
      host.send({ type: "referrers", ...this.#referrers });
      if (refresh) return;
    }
    const places = cards.map(({ source, path, start }) => ({
      uri: uris[source],
      path,
      offset: start,
    }));
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
    const places = [];
    for (const file of files.values()) {
      if (!active()) return;
      const own = file.uri === focus.uri;
      if (own && file.text !== text) continue;
      const enabled = own ? cfg : file.cfg;
      let targets;
      try {
        if (!file.targets || file.targets.cfg !== enabled) {
          file.targets = {
            cfg: enabled,
            offsets: engine.relationshipTargets(file.text, enabled),
          };
        }
        targets = await file.targets.offsets;
      } catch {
        // A text being edited can fail to parse while the other files are usable.
        file.targets = undefined;
        continue;
      }
      const known = new Set(file.offsets);
      for (const offset of targets) {
        if (!known.has(offset)) places.push({ uri: file.uri, offset });
      }
    }
    if (!places.length || !active()) return;
    this.#definitions = { id: ++next, places };
    host.send({
      type: "definitions",
      uri: focus.uri,
      offset: focus.offset,
      ...this.#definitions,
    });
  }

  /** Only the latest request of each kind can change the drawing. */
  receive(message) {
    if (message.type === "referrers" && message.id === this.#referrers?.id) {
      this.#files = new Map(
        message.files.map((source) => {
          const previous = this.#files.get(source.uri);
          const file =
            previous?.text === source.text && previous.cfg === source.cfg
              ? previous
              : { ...source };
          // A new reference search can reuse a worker parse of the same source.
          file.offsets = [...source.offsets];
          return [file.uri, file];
        }),
      );
      this.#definitions = null;
      // A new search can bring a previously excluded declaration into the workspace.
      this.#placements.clear();
      this.#places = [];
      this.#modulesId = 0;
    } else if (
      message.type === "definitions" &&
      message.id === this.#definitions?.id
    ) {
      const { places } = this.#definitions;
      this.#definitions = null;
      let found = false;
      places.forEach(({ uri, offset }, index) => {
        if (message.matches[index] === true) {
          this.#files.get(uri).offsets.push(offset);
          found = true;
        }
      });
      if (!found) return false;
    } else if (message.type === "modules" && message.id === this.#modulesId) {
      this.#modules.clear();
      this.#places.forEach(({ uri, path }, index) => {
        const module = message.modules[index];
        if (typeof module === "string") {
          const paths = this.#modules.get(uri) ?? Object.create(null);
          paths[path] = module;
          this.#modules.set(uri, paths);
        }
        const placements = message.placements?.[index];
        if (Array.isArray(placements)) {
          const paths = this.#placements.get(uri) ?? Object.create(null);
          paths[path] = placements;
          this.#placements.set(uri, paths);
        } else if (this.#placements.has(uri)) {
          delete this.#placements.get(uri)[path];
        }
      });
    } else {
      return false;
    }
    ++this.version;
    return true;
  }
}
