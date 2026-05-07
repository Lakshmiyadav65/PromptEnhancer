import { StatusIndicator } from "./components/StatusIndicator";
import { Settings } from "./components/Settings";

function App() {
  const hash = window.location.hash;
  if (hash === "#/status") return <StatusIndicator />;
  if (hash === "#/settings") return <Settings />;

  return (
    <main style={{ padding: 16, fontFamily: "system-ui, sans-serif" }}>
      <h1>PromptForge</h1>
      <p>Use the tray icon to open Settings. Press your hotkey to enhance any selected text.</p>
    </main>
  );
}

export default App;
