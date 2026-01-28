import { Component, Show, createSignal, onMount } from "solid-js";
import { useSettings, loadSettings } from "./stores/settings";
import { Task, TaskMessage, AgentEvent, Document, listTasks, createTask, deleteTask, runTaskAgent, getTask, getTaskMessages } from "./lib/tauri-api";
import { MCPAppInstance, createMCPAppInstance } from "./lib/mcp-api";
import AgentMain from "./components/AgentMain";
import Settings from "./components/Settings";
import SkillsList from "./components/SkillsList";
import MCPSettings from "./components/MCPSettings";
import TaskSidebar from "./components/TaskSidebar";
import TaskPanel from "./components/TaskPanel";
import CapturePanel from "./components/CapturePanel";
import BrowserPanel from "./components/BrowserPanel";
import WorkStreamPanel from "./components/WorkStreamPanel";
import DocEditor from "./components/DocEditor";
import ChatWidget from "./components/ChatWidget";
import DataPanelsDock from "./components/DataPanels/DataPanelsDock";
import ResizablePanels from "./components/ResizablePanels";
import { showDataPanels, setShowDataPanels, loadPanelState } from "./stores/dataPanels";
import { useDocs } from "./stores/docs";
import { createToolTracker } from "./stores/workstream";
import { captureAIExchange } from "./lib/capture-api";

interface ToolExecution {
  id: number;
  tool: string;
  status: "running" | "completed" | "error";
}

const App: Component = () => {
  const { showSettings, toggleSettings, isLoading } = useSettings();

  // UI state
  const [showSkills, setShowSkills] = createSignal(false);
  const [showMCP, setShowMCP] = createSignal(false);
  const [showCapturePanel, setShowCapturePanel] = createSignal(false);
  const [showBrowserPanel, setShowBrowserPanel] = createSignal(false);
  const [showActivityPanel, setShowActivityPanel] = createSignal(false);
  const [showDocEditor, setShowDocEditor] = createSignal(false);
  const [activeDocId, setActiveDocId] = createSignal<string | null>(null);

  // WorkStream tool tracking
  const { trackToolStart, trackToolEnd } = createToolTracker();

  // Docs hooks
  const { openDocument } = useDocs();

  // Task state
  const [tasks, setTasks] = createSignal<Task[]>([]);
  const [activeTask, setActiveTask] = createSignal<Task | null>(null);
  const [taskMessages, setTaskMessages] = createSignal<TaskMessage[]>([]);
  const [isRunning, setIsRunning] = createSignal(false);
  const [toolExecutions, setToolExecutions] = createSignal<ToolExecution[]>([]);
  const [currentText, setCurrentText] = createSignal("");

  // MCP Apps state
  const [activeApps, setActiveApps] = createSignal<MCPAppInstance[]>([]);

  onMount(async () => {
    await loadSettings();
    await refreshTasks();
    await loadPanelState();
  });

  const toggleSkills = () => {
    setShowSkills(!showSkills());
    // Close other panels if open
    if (showSettings()) {
      toggleSettings();
    }
    setShowMCP(false);
    setShowDocEditor(false);
    setActiveDocId(null);
    setShowDataPanels(false);
  };

  const toggleMCP = () => {
    setShowMCP(!showMCP());
    // Close other panels if open
    if (showSettings()) {
      toggleSettings();
    }
    setShowSkills(false);
    setShowDocEditor(false);
    setActiveDocId(null);
    setShowDataPanels(false);
  };

  const handleToggleSettings = () => {
    // Close other panels if open
    setShowSkills(false);
    setShowMCP(false);
    setShowDocEditor(false);
    setActiveDocId(null);
    setShowDataPanels(false);
    toggleSettings();
  };

  const toggleDataPanels = () => {
    const newState = !showDataPanels();
    setShowDataPanels(newState);
    if (newState) {
      // Close other main panel views
      setShowDocEditor(false);
      setActiveDocId(null);
      if (showSettings()) toggleSettings();
      setShowSkills(false);
      setShowMCP(false);
    }
  };

  const toggleCapturePanel = () => {
    setShowCapturePanel(!showCapturePanel());
    if (showCapturePanel()) {
      setShowBrowserPanel(false);
    }
  };

  const toggleBrowserPanel = () => {
    setShowBrowserPanel(!showBrowserPanel());
    if (showBrowserPanel()) {
      setShowCapturePanel(false);
      setShowActivityPanel(false);
    }
  };

  const toggleActivityPanel = () => {
    setShowActivityPanel(!showActivityPanel());
    if (showActivityPanel()) {
      setShowCapturePanel(false);
      setShowBrowserPanel(false);
    }
  };

  const toggleDocEditor = () => {
    const newState = !showDocEditor();
    setShowDocEditor(newState);
    if (newState) {
      // Close other main panel views
      setShowDataPanels(false);
      if (showSettings()) toggleSettings();
      setShowSkills(false);
      setShowMCP(false);
    } else {
      setActiveDocId(null);
    }
  };

  const handleSelectDoc = async (doc: Document) => {
    setActiveDocId(doc.id);
    setShowDocEditor(true);
    // Close other main panel views
    setShowDataPanels(false);
    if (showSettings()) toggleSettings();
    setShowSkills(false);
    setShowMCP(false);
    await openDocument(doc.id);
  };

  const refreshTasks = async () => {
    const taskList = await listTasks();
    setTasks(taskList);
  };

  const handleNewTask = async (title: string, description: string, projectPath?: string) => {
    const task = await createTask(title, description, projectPath);
    setActiveTask(task);

    // Add user message to local state immediately for display
    const tempUserMessage: TaskMessage = {
      id: `temp-${Date.now()}`,
      task_id: task.id,
      role: "user",
      content: description,
      timestamp: Date.now(),
    };
    setTaskMessages([tempUserMessage]);
    await refreshTasks();

    // Start the agent
    setIsRunning(true);
    setToolExecutions([]);
    setCurrentText("");

    try {
      await runTaskAgent(
        {
          task_id: task.id,
          message: description,
          project_path: projectPath,
          max_turns: 50,
        },
        handleAgentEvent
      );
    } catch (err) {
      console.error("Task error:", err);
    } finally {
      setIsRunning(false);
      // Refresh task to get final state
      const updated = await getTask(task.id);
      if (updated) {
        setActiveTask(updated);
      }
      // Reload messages to show saved conversation
      const messages = await getTaskMessages(task.id);
      setTaskMessages(messages);
      await refreshTasks();
    }
  };

  const handleAgentEvent = async (event: AgentEvent) => {
    console.log("Agent event:", event);

    switch (event.type) {
      case "text":
        setCurrentText(event.content);
        break;
      case "plan":
        // Update active task with plan
        setActiveTask((prev) => {
          if (!prev) return prev;
          return {
            ...prev,
            plan: event.steps.map((s) => ({
              step: s.step,
              description: s.description,
              status: "pending" as const,
            })),
          };
        });
        break;
      case "step_start":
        setActiveTask((prev) => {
          if (!prev || !prev.plan) return prev;
          return {
            ...prev,
            current_step: event.step,
            plan: prev.plan.map((s) =>
              s.step === event.step ? { ...s, status: "running" as const } : s
            ),
          };
        });
        break;
      case "step_done":
        setActiveTask((prev) => {
          if (!prev || !prev.plan) return prev;
          return {
            ...prev,
            plan: prev.plan.map((s) =>
              s.step === event.step ? { ...s, status: "completed" as const } : s
            ),
          };
        });
        break;
      case "tool_start":
        setToolExecutions((prev) => [
          ...prev,
          { id: Date.now(), tool: event.tool, status: "running" },
        ]);
        // Log activity event for tool start
        trackToolStart(event.tool, event.input);
        break;
      case "tool_end":
        setToolExecutions((prev) => {
          const updated = [...prev];
          const last = updated.findLast((t: ToolExecution) => t.tool === event.tool && t.status === "running");
          if (last) {
            last.status = event.success ? "completed" : "error";
          }
          return updated;
        });
        // Log activity event for tool end
        trackToolEnd(event.tool, event.result?.slice(0, 100), event.success);
        break;
      case "done":
        setActiveTask((prev) => {
          if (!prev) return prev;
          return { ...prev, status: "completed" };
        });
        // Capture AI exchange for source linking
        {
          const msgs = taskMessages();
          const lastUserMsg = [...msgs].reverse().find((m) => m.role === "user");
          const aiResponse = currentText();
          if (lastUserMsg && aiResponse) {
            // Get model from settings (default to claude)
            const settings = useSettings();
            const model = settings.settings()?.model || "claude";
            captureAIExchange(
              lastUserMsg.content,
              aiResponse,
              model,
              activeDocId() || undefined
            ).catch(console.error);
          }
        }
        break;
      case "error":
        setActiveTask((prev) => {
          if (!prev) return prev;
          return { ...prev, status: "failed" };
        });
        break;
    }
  };

  const handleSelectTask = async (task: Task) => {
    setActiveTask(task);
    setCurrentText("");
    setToolExecutions([]);
    // Load conversation history for this task
    const messages = await getTaskMessages(task.id);
    setTaskMessages(messages);
  };

  // Continue conversation with existing task
  const handleContinueTask = async (message: string, projectPath?: string) => {
    const task = activeTask();
    if (!task) return;

    // Add user message to local state immediately for display
    const tempUserMessage: TaskMessage = {
      id: `temp-${Date.now()}`,
      task_id: task.id,
      role: "user",
      content: message,
      timestamp: Date.now(),
    };
    setTaskMessages((prev) => [...prev, tempUserMessage]);

    setIsRunning(true);
    setToolExecutions([]);
    setCurrentText("");

    try {
      await runTaskAgent(
        {
          task_id: task.id,
          message,
          project_path: projectPath || task.project_path || undefined,
          max_turns: 50,
        },
        handleAgentEvent
      );
    } catch (err) {
      console.error("Task error:", err);
    } finally {
      setIsRunning(false);
      // Refresh task to get final state
      const updated = await getTask(task.id);
      if (updated) {
        setActiveTask(updated);
      }
      // Reload messages to show saved conversation
      const messages = await getTaskMessages(task.id);
      setTaskMessages(messages);
      await refreshTasks();
    }
  };

  // Clear active task to start a new one
  const handleNewConversation = () => {
    setActiveTask(null);
    setTaskMessages([]);
    setCurrentText("");
    setToolExecutions([]);
  };

  // Delete a task
  const handleDeleteTask = async (taskId: string) => {
    await deleteTask(taskId);
    // If we deleted the active task, clear it
    if (activeTask()?.id === taskId) {
      setActiveTask(null);
      setTaskMessages([]);
      setCurrentText("");
      setToolExecutions([]);
      setActiveApps([]);
    }
    await refreshTasks();
  };

  // MCP Apps handlers
  const handleCloseApp = (appId: string) => {
    setActiveApps((prev) => prev.filter((app) => app.id !== appId));
  };

  const handleToolWithUI = async (serverId: string, toolName: string, result: unknown) => {
    try {
      const instance = await createMCPAppInstance(serverId, toolName, result);
      setActiveApps((prev) => [...prev, instance]);
    } catch (err) {
      console.error("Failed to create MCP App instance:", err);
    }
  };

  return (
    <div class="app agent-layout">
      <Show when={!isLoading()} fallback={<LoadingScreen />}>
        <TaskSidebar
          tasks={tasks()}
          activeTaskId={activeTask()?.id || null}
          onSelectTask={handleSelectTask}
          onDeleteTask={handleDeleteTask}
          onSettingsClick={handleToggleSettings}
          onSkillsClick={toggleSkills}
          onMCPClick={toggleMCP}
          onDataClick={toggleDataPanels}
          onCaptureClick={toggleCapturePanel}
          onBrowserClick={toggleBrowserPanel}
          onActivityClick={toggleActivityPanel}
          onDocClick={toggleDocEditor}
          onSelectDoc={handleSelectDoc}
          showDataPanels={showDataPanels()}
          showCapturePanel={showCapturePanel()}
          showBrowserPanel={showBrowserPanel()}
          showActivityPanel={showActivityPanel()}
          showDocEditor={showDocEditor()}
          activeDocId={activeDocId()}
        />
        <ResizablePanels
          defaultRightWidth={350}
          minRightWidth={250}
          maxRightWidth={700}
          left={
            <main class="main-content">
              <Show when={showSettings()}>
                <Settings />
              </Show>
              <Show when={showSkills()}>
                <SkillsList />
              </Show>
              <Show when={showMCP()}>
                <MCPSettings onClose={() => setShowMCP(false)} />
              </Show>
              <Show when={showDocEditor()}>
                <DocEditor
                  docId={activeDocId()}
                  onClose={() => {
                    setShowDocEditor(false);
                    setActiveDocId(null);
                  }}
                />
              </Show>
              <Show when={showDataPanels()}>
                <DataPanelsDock />
              </Show>
              <Show when={!showSettings() && !showSkills() && !showMCP() && !showDocEditor() && !showDataPanels()}>
                <AgentMain
                  onNewTask={handleNewTask}
                  onContinueTask={handleContinueTask}
                  onNewConversation={handleNewConversation}
                  currentText={currentText()}
                  isRunning={isRunning()}
                  activeTask={activeTask()}
                  messages={taskMessages()}
                  activeApps={activeApps()}
                  onCloseApp={handleCloseApp}
                  onToolWithUI={handleToolWithUI}
                />
              </Show>
            </main>
          }
          right={
            <aside class="task-panel-container" style={{ display: "flex", "flex-direction": "column" }}>
              <div style={{ flex: 1, overflow: "hidden", position: "relative" }}>
                <Show
                  when={showBrowserPanel()}
                  fallback={
                    <Show
                      when={showActivityPanel()}
                      fallback={
                        <Show
                          when={showCapturePanel()}
                          fallback={
                            <TaskPanel
                              task={activeTask()}
                              isRunning={isRunning()}
                              toolExecutions={toolExecutions()}
                            />
                          }
                        >
                          <CapturePanel
                            docId={activeDocId() || activeTask()?.id || null}
                            onClose={() => setShowCapturePanel(false)}
                          />
                        </Show>
                      }
                    >
                      <WorkStreamPanel
                        contextId={activeDocId() || activeTask()?.id || undefined}
                        contextType={activeDocId() ? "document" : activeTask() ? "task" : undefined}
                        onClose={() => setShowActivityPanel(false)}
                      />
                    </Show>
                  }
                >
                  <BrowserPanel
                    docId={activeDocId() || activeTask()?.id || null}
                    onClose={() => setShowBrowserPanel(false)}
                  />
                </Show>
              </div>
              <Show when={showDocEditor() || showDataPanels()}>
                <div style={{ height: "400px", "flex-shrink": 0 }}>
                  <ChatWidget
                    messages={taskMessages()}
                    isRunning={isRunning()}
                    onSendMessage={(msg) => {
                      if (activeTask()) {
                        handleContinueTask(msg);
                      } else {
                        const title = msg.split("\n")[0].slice(0, 30) + "...";
                        handleNewTask(title, msg);
                      }
                    }}
                  />
                </div>
              </Show>
            </aside>
          }
        />
      </Show>
    </div>
  );
};

const LoadingScreen: Component = () => (
  <div class="loading-screen">
    <div class="loading-content">
      <h1>Kuse Cowork</h1>
      <p>Loading...</p>
    </div>
  </div>
);

export default App;
