// Nav layers (pure): every mounted screen, overlay and dialog registers a handler with a z level;
// a `ui.nav` event goes only to the top-most one (highest z, then the most recently added), so a
// dialog shadows its screen regardless of React's effect order or hot reloads.

export type Layer<E> = { z: number; seq: number; handler: { current: (event: E) => void } };

export type NavStack<E> = {
  /** Adds a layer; returns its removal. */
  add(z: number, handler: { current: (event: E) => void }): () => void;
  /** Delivers `event` to the top-most layer; false when no layer is mounted. */
  dispatch(event: E): boolean;
  size(): number;
};

export function createNavStack<E>(): NavStack<E> {
  const layers: Layer<E>[] = [];
  let seq = 0;
  return {
    add(z, handler) {
      const layer = { z, seq: seq++, handler };
      layers.push(layer);
      return () => {
        const i = layers.indexOf(layer);
        if (i >= 0) layers.splice(i, 1);
      };
    },
    dispatch(event) {
      let top: Layer<E> | undefined;
      for (const layer of layers) {
        if (!top || layer.z > top.z || (layer.z === top.z && layer.seq > top.seq)) top = layer;
      }
      top?.handler.current(event);
      return top !== undefined;
    },
    size: () => layers.length,
  };
}
