import { Component, Show } from "solid-js";
import { Suggestion } from "../stores/traces";
import "./SuggestionCard.css";

interface SuggestionCardProps {
  suggestion: Suggestion;
  onApprove: () => void;
  onReject: () => void;
  onDismiss: () => void;
  isApplying?: boolean;
}

const SuggestionCard: Component<SuggestionCardProps> = (props) => {
  const getTypeIcon = (type: string): string => {
    switch (type) {
      case "edit":
        return "P";
      case "add_section":
        return "+";
      case "search":
        return "S";
      case "refactor":
        return "R";
      default:
        return "*";
    }
  };

  const getTypeLabel = (type: string): string => {
    switch (type) {
      case "edit":
        return "Edit";
      case "add_section":
        return "Add Section";
      case "search":
        return "Search";
      case "refactor":
        return "Refactor";
      default:
        return "Suggestion";
    }
  };

  const isPending = () => props.suggestion.status === "pending";
  const isApproved = () => props.suggestion.status === "approved";
  const isRejected = () => props.suggestion.status === "rejected";

  return (
    <div class={`suggestion-card-standalone ${props.suggestion.status}`}>
      <div class="suggestion-header">
        <div class="suggestion-type-badge">
          <span class="suggestion-type-icon">{getTypeIcon(props.suggestion.suggestion_type)}</span>
          <span class="suggestion-type-label">{getTypeLabel(props.suggestion.suggestion_type)}</span>
        </div>
        <Show when={!isPending()}>
          <span class={`suggestion-status-badge ${props.suggestion.status}`}>
            {isApproved() ? "Approved" : "Rejected"}
          </span>
        </Show>
      </div>

      <div class="suggestion-body">
        <h4 class="suggestion-title">{props.suggestion.title}</h4>
        <p class="suggestion-description">{props.suggestion.description}</p>

        <Show when={props.suggestion.payload && Object.keys(props.suggestion.payload).length > 0}>
          <div class="suggestion-preview">
            <details>
              <summary>View Details</summary>
              <pre>{JSON.stringify(props.suggestion.payload, null, 2)}</pre>
            </details>
          </div>
        </Show>
      </div>

      <Show when={isPending()}>
        <div class="suggestion-actions">
          <button
            class="suggestion-action-btn approve"
            onClick={props.onApprove}
            disabled={props.isApplying}
          >
            {props.isApplying ? "Applying..." : "Approve"}
          </button>
          <button
            class="suggestion-action-btn reject"
            onClick={props.onReject}
            disabled={props.isApplying}
          >
            Reject
          </button>
          <button
            class="suggestion-action-btn dismiss"
            onClick={props.onDismiss}
            disabled={props.isApplying}
          >
            Dismiss
          </button>
        </div>
      </Show>
    </div>
  );
};

export default SuggestionCard;
