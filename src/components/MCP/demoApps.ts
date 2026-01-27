import { MCPAppInstance } from "../../lib/mcp-api";

export const DEMO_APPS: Record<string, MCPAppInstance> = {
  chartViewer: {
    id: "demo-chart",
    server_id: "demo",
    tool_name: "chart_viewer",
    html_content: `<!DOCTYPE html>
<html>
<head>
  <style>
    body { font-family: system-ui, sans-serif; padding: 20px; margin: 0; background: #f8f9fa; }
    .chart-container { background: white; border-radius: 12px; padding: 20px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }
    h2 { margin: 0 0 16px 0; color: #333; }
    .bar-chart { display: flex; align-items: flex-end; gap: 12px; height: 150px; padding: 10px 0; }
    .bar { background: linear-gradient(180deg, #007bff, #0056b3); border-radius: 4px 4px 0 0; min-width: 40px; transition: height 0.3s; }
    .bar:hover { background: linear-gradient(180deg, #0056b3, #003d80); }
    .bar-label { text-align: center; font-size: 12px; color: #666; margin-top: 8px; }
    .bar-wrapper { display: flex; flex-direction: column; align-items: center; }
    button { background: #007bff; color: white; border: none; padding: 10px 20px; border-radius: 6px; cursor: pointer; margin-top: 16px; }
    button:hover { background: #0056b3; }
  </style>
</head>
<body>
  <div class="chart-container">
    <h2>Sales Data Visualization</h2>
    <div class="bar-chart">
      <div class="bar-wrapper"><div class="bar" style="height: 80px;"></div><span class="bar-label">Jan</span></div>
      <div class="bar-wrapper"><div class="bar" style="height: 120px;"></div><span class="bar-label">Feb</span></div>
      <div class="bar-wrapper"><div class="bar" style="height: 90px;"></div><span class="bar-label">Mar</span></div>
      <div class="bar-wrapper"><div class="bar" style="height: 140px;"></div><span class="bar-label">Apr</span></div>
      <div class="bar-wrapper"><div class="bar" style="height: 100px;"></div><span class="bar-label">May</span></div>
    </div>
    <button onclick="alert('This is a demo! In a real app, this would call a tool.')">Refresh Data</button>
  </div>
</body>
</html>`,
    tool_result: { data: [80, 120, 90, 140, 100] },
    permissions: ["allow-scripts"],
  },
  dataForm: {
    id: "demo-form",
    server_id: "demo",
    tool_name: "data_form",
    html_content: `<!DOCTYPE html>
<html>
<head>
  <style>
    body { font-family: system-ui, sans-serif; padding: 20px; margin: 0; background: #f0f4f8; }
    .form-container { background: white; border-radius: 12px; padding: 24px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); max-width: 400px; }
    h2 { margin: 0 0 20px 0; color: #333; }
    .form-group { margin-bottom: 16px; }
    label { display: block; margin-bottom: 6px; font-weight: 500; color: #555; }
    input, select { width: 100%; padding: 10px; border: 1px solid #ddd; border-radius: 6px; font-size: 14px; box-sizing: border-box; }
    input:focus, select:focus { outline: none; border-color: #007bff; box-shadow: 0 0 0 3px rgba(0,123,255,0.1); }
    button { background: #28a745; color: white; border: none; padding: 12px 24px; border-radius: 6px; cursor: pointer; width: 100%; font-size: 14px; font-weight: 500; }
    button:hover { background: #218838; }
  </style>
</head>
<body>
  <div class="form-container">
    <h2>Quick Entry Form</h2>
    <div class="form-group">
      <label>Name</label>
      <input type="text" placeholder="Enter name..." />
    </div>
    <div class="form-group">
      <label>Category</label>
      <select>
        <option>Select category...</option>
        <option>Sales</option>
        <option>Marketing</option>
        <option>Engineering</option>
      </select>
    </div>
    <div class="form-group">
      <label>Amount</label>
      <input type="number" placeholder="0.00" />
    </div>
    <button onclick="alert('Demo: Form would submit via MCP tool call!')">Submit</button>
  </div>
</body>
</html>`,
    tool_result: {},
    permissions: ["allow-scripts", "allow-forms"],
  },
};
