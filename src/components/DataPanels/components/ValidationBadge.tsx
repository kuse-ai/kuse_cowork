import { Component } from "solid-js";
import type { ValidationBadgeProps } from "../types";

export const ValidationBadge: Component<ValidationBadgeProps> = (props) => {
  const icon = () => props.type === "error" ? "\u{2716}" : "\u{26A0}";

  return (
    <span
      class={`validation-badge ${props.type}`}
      title={props.message}
    >
      {icon()}
    </span>
  );
};

// CSS
const styles = `
.validation-badge {
  position: absolute;
  top: 0;
  right: 0;
  padding: 0 2px;
  font-size: 0.5rem;
  cursor: help;
}

.validation-badge.error {
  color: #dc3545;
}

.validation-badge.warning {
  color: #f59e0b;
}
`;

// Inject styles
if (typeof document !== "undefined") {
  const styleEl = document.createElement("style");
  styleEl.textContent = styles;
  document.head.appendChild(styleEl);
}

export default ValidationBadge;
