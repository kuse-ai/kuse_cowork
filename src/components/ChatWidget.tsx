import { Component, For, createSignal, createEffect, useRef } from "solid-js";
import { TaskMessage } from "../lib/tauri-api";
import "./ChatWidget.css";

interface ChatWidgetProps {
  messages: TaskMessage[];
  onSendMessage: (message: string) => void;
  isRunning: boolean;
  placeholder?: string;
}

const ChatWidget: Component<ChatWidgetProps> = (props) => {
  const [input, setInput] = createSignal("");
  let messagesContainer: HTMLDivElement | undefined;

  // Auto-scroll to bottom
  createEffect(() => {
    if (props.messages.length && messagesContainer) {
      messagesContainer.scrollTop = messagesContainer.scrollHeight;
    }
  });

  const handleSubmit = (e: Event) => {
    e.preventDefault();
    if (!input().trim() || props.isRunning) return;
    props.onSendMessage(input());
    setInput("");
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e);
    }
  };

  return (
    <div class="chat-widget">
      <div class="chat-widget-header">
        <h3>Agent Chat</h3>
      </div>
      
      <div class="chat-widget-messages" ref={messagesContainer}>
        <For each={props.messages}>
          {(msg) => (
            <div class={`chat-bubble ${msg.role}`}>
              <div class="chat-content">{msg.content}</div>
            </div>
          )}
        </For>
        {props.isRunning && (
          <div class="chat-bubble assistant streaming">
            <div class="typing-indicator">
              <span>.</span><span>.</span><span>.</span>
            </div>
          </div>
        )}
      </div>

      <div class="chat-widget-input">
        <textarea
          value={input()}
          onInput={(e) => setInput(e.currentTarget.value)}
          onKeyDown={handleKeyDown}
          placeholder={props.placeholder || "Ask the agent..."}
          rows={1}
          disabled={props.isRunning}
        />
        <button 
          class="chat-send-btn" 
          onClick={handleSubmit} 
          disabled={!input().trim() || props.isRunning}
        >
          ↑
        </button>
      </div>
    </div>
  );
};

export default ChatWidget;
