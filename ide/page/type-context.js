/** Requests keep distinct ids when the page moves to another type or text. */
let next = 0;

/** The host's references for one drawing's type, text and cfg. */
export class TypeContext {
  /** Each file the engine finds references in, read in the crate the engine reads it in. */
  #files = [];
  /** Where the focus's text stands in the workspace's module tree. */
  #place;
  #referrers = null;
  #waiting = false;
  #refreshing = false;
  version = 0;

  constructor(path, cfg, start) {
    this.path = path;
    this.start = start;
    this.cfg = cfg;
  }

  /** The bindings number their sources from the focus, then each other file. */
  sources(uri, text) {
    const own = this.#files.find((file) => file.uri === uri);
    const others = this.#files.filter((file) => file.uri !== uri);
    return {
      uris: [uri, ...others.map((file) => file.uri)],
      texts: [text, ...others.map((file) => file.text)],
      context: {
        // The focus's references count only in the text they were found in.
        offsets: own?.text === text ? own.offsets : [],
        place: this.#place,
        files: others.map(({ text, cfg, offsets, place }) => ({
          text,
          cfg,
          offsets,
          place,
        })),
      },
    };
  }

  /** A refresh keeps its finished diagram until the new references arrive. */
  canPublish() {
    if (this.#refreshing && this.#waiting) return false;
    this.#refreshing = false;
    return true;
  }

  /** Asks again, when the engine may have analyzed more. */
  refresh(host) {
    if (this.#referrers) {
      this.#refreshing = true;
      this.#request(host, this.#referrers.uri, this.#referrers.offset);
    }
  }

  /** Asks once for the references to the focus named at `offset` of `uri`. */
  ask(host, uri, offset) {
    if (!this.#referrers) this.#request(host, uri, offset);
  }

  #request(host, uri, offset) {
    this.#referrers = { id: ++next, uri, offset };
    this.#waiting = true;
    host.send({ type: "referrers", ...this.#referrers });
  }

  /** Only the latest answer can change the drawing. */
  receive(message) {
    if (message.type !== "referrers" || message.id !== this.#referrers?.id)
      return false;
    this.#waiting = false;
    this.#files = message.files;
    this.#place = message.focus?.place;
    if (message.focus?.cfg !== undefined) this.cfg = message.focus.cfg;
    ++this.version;
    return true;
  }
}
