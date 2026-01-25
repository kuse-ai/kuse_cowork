import { Component, Show, createSignal, onMount } from "solid-js";
import { useSettings, loadSettings } from "./stores/settings";
import { Task, TaskMessage, AgentEvent, listTasks, createTask, deleteTask, runTaskAgent, getTask, getTaskMessages } from "./lib/tauri-api";
import AgentMain from "./components/AgentMain";
import Settings from "./components/Settings";
import SkillsList from "./components/SkillsList";
import MCPSettings from "./components/MCPSettings";
import TaskSidebar from "./components/TaskSidebar";
import TaskPanel from "./components/TaskPanel";
import DataPanelsDock from "./components/DataPanels/DataPanelsDock";
import { showDataPanels, setShowDataPanels, loadPanelState } from "./stores/dataPanels";

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

  // Task state
  const [tasks, setTasks] = createSignal<Task[]>([]);
  const [activeTask, setActiveTask] = createSignal<Task | null>(null);
  const [taskMessages, setTaskMessages] = createSignal<TaskMessage[]>([]);
  const [isRunning, setIsRunning] = createSignal(false);
  const [toolExecutions, setToolExecutions] = createSignal<ToolExecution[]>([]);
  const [currentText, setCurrentText] = createSignal("");

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
    if (showMCP()) {
      setShowMCP(false);
    }
  };

  const toggleMCP = () => {
    setShowMCP(!showMCP());
    // Close other panels if open
    if (showSettings()) {
      toggleSettings();
    }
    if (showSkills()) {
      setShowSkills(false);
    }
  };

  const handleToggleSettings = () => {
    // Close other panels if open
    if (showSkills()) {
      setShowSkills(false);
    }
    if (showMCP()) {
      setShowMCP(false);
    }
    toggleSettings();
  };

  const toggleDataPanels = () => {
    setShowDataPanels(!showDataPanels());
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
        break;
      case "done":
        setActiveTask((prev) => {
          if (!prev) return prev;
          return { ...prev, status: "completed" };
        });
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
    }
    await refreshTasks();
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
          showDataPanels={showDataPanels()}
        />
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
          <Show when={!showSettings() && !showSkills() && !showMCP()}>
            <AgentMain
              onNewTask={handleNewTask}
              onContinueTask={handleContinueTask}
              onNewConversation={handleNewConversation}
              currentText={currentText()}
              isRunning={isRunning()}
              activeTask={activeTask()}
              messages={taskMessages()}
            />
          </Show>
        </main>
        <aside class="task-panel-container">
          <Show
            when={showDataPanels()}
            fallback={
              <TaskPanel
                task={activeTask()}
                isRunning={isRunning()}
                toolExecutions={toolExecutions()}
              />
            }
          >
            <DataPanelsDock />
          </Show>
        </aside>
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
