import { Component, JSX, createSignal, onCleanup } from "solid-js";
import "./ResizablePanels.css";

interface ResizablePanelsProps {
  left: JSX.Element;
  right: JSX.Element;
  defaultRightWidth?: number;
  minRightWidth?: number;
  maxRightWidth?: number;
}

const ResizablePanels: Component<ResizablePanelsProps> = (props) => {
  const [rightWidth, setRightWidth] = createSignal(props.defaultRightWidth || 350);
  const [isDragging, setIsDragging] = createSignal(false);

  const minWidth = props.minRightWidth || 250;
  const maxWidth = props.maxRightWidth || 600;

  let containerRef: HTMLDivElement | undefined;

  const handleMouseDown = (e: MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (!isDragging() || !containerRef) return;

    const containerRect = containerRef.getBoundingClientRect();
    const newRightWidth = containerRect.right - e.clientX;

    // Clamp to min/max
    const clampedWidth = Math.min(Math.max(newRightWidth, minWidth), maxWidth);
    setRightWidth(clampedWidth);
  };

  const handleMouseUp = () => {
    setIsDragging(false);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  };

  // Add global event listeners when dragging
  const startDrag = (e: MouseEvent) => {
    handleMouseDown(e);
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  };

  onCleanup(() => {
    document.removeEventListener("mousemove", handleMouseMove);
    document.removeEventListener("mouseup", handleMouseUp);
  });

  return (
    <div ref={containerRef} class="resizable-panels">
      <div class="resizable-panels-left">
        {props.left}
      </div>
      <div
        class={`resizable-panels-divider ${isDragging() ? "dragging" : ""}`}
        onMouseDown={startDrag}
      >
        <div class="divider-handle" />
      </div>
      <div
        class="resizable-panels-right"
        style={{ width: `${rightWidth()}px` }}
      >
        {props.right}
      </div>
    </div>
  );
};

export default ResizablePanels;
