// List (a scroll container that keeps the focused row in view) and Carousel (a strip that slides
// to centre the focused item).
import type { ReactNode } from "react";

import { followScroll } from "../nav/focus";
import { spring } from "../theme/motion";

type ListProps = {
  index: number;
  rows: number;
  rowHeight: number;
  gap?: number;
  height: number;
  children: ReactNode;
};

/** A vertical list; the mouse wheel scrolls it too. */
export function List({ index, rows, rowHeight, gap = 6, height, children }: ListProps) {
  return (
    <node
      scrollTop={followScroll(index, rowHeight, gap, rows, height)}
      scrollStep={rowHeight + gap}
      style={{
        height,
        flexDirection: "column",
        gap,
        overflowY: "scroll",
        scrollbar: "none",
        transition: { scroll: { duration: 160, easing: "easeOut" } },
      }}
    >
      {children}
    </node>
  );
}

type CarouselProps = {
  count: number;
  index: number;
  itemWidth: number;
  gap?: number;
  /** Width of the visible window. */
  width: number;
  height: number;
  render: (i: number) => ReactNode;
};

/** A horizontal strip centred on `index`. */
export function Carousel({ count, index, itemWidth, gap = 18, width, height, render }: CarouselProps) {
  const offset = width / 2 - (index * (itemWidth + gap) + itemWidth / 2);
  return (
    <node style={{ width, height, overflowX: "clip", overflowY: "visible" }}>
      <node
        style={{
          positionType: "absolute",
          left: 0,
          top: 0,
          height,
          flexDirection: "row",
          alignItems: "center",
          gap,
          transform: { translateX: offset },
          transition: { transform: spring.soft },
        }}
      >
        {Array.from({ length: count }, (_, i) => (
          <node key={i} style={{ width: itemWidth, flexShrink: 0 }}>
            {render(i)}
          </node>
        ))}
      </node>
    </node>
  );
}
