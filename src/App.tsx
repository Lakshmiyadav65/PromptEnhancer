import { StatusIndicator } from "./components/StatusIndicator";

function App() {
  // Hash-based routing: status window loads index.html#/status, main window loads index.html
  if (window.location.hash === "#/status") {
    return <StatusIndicator />;
  }

  return (
    <main style={{ padding: 16, fontFamily: "system-ui, sans-serif" }}>
      <h1>PromptForge</h1>
      <p>Settings window comes in Phase 6.</p>
    </main>
  );
}

export default App;
