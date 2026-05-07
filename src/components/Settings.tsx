import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./Settings.css";

type ApiKeyStatus = { from_env: boolean; from_keychain: boolean };
type ConnectionTest = { ok: boolean; latency_ms: number; message: string };
type Msg = { ok: boolean; text: string } | null;

export function Settings() {
  const [keyStatus, setKeyStatus] = useState<ApiKeyStatus | null>(null);
  const [hotkey, setHotkey] = useState("CommandOrControl+Alt+E");
  const [keyInput, setKeyInput] = useState("");
  const [keyMsg, setKeyMsg] = useState<Msg>(null);
  const [testMsg, setTestMsg] = useState<Msg>(null);
  const [hotkeyMsg, setHotkeyMsg] = useState<Msg>(null);
  const [recording, setRecording] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    refresh();
  }, []);

  async function refresh() {
    try {
      const status = await invoke<ApiKeyStatus>("api_key_status");
      setKeyStatus(status);
      const hk = await invoke<string>("get_hotkey");
      setHotkey(hk);
    } catch (e) {
      console.error("refresh failed:", e);
    }
  }

  async function saveKey() {
    if (!keyInput.trim()) return;
    setBusy("key");
    setKeyMsg(null);
    try {
      await invoke("save_api_key", { key: keyInput.trim() });
      setKeyInput("");
      setKeyMsg({ ok: true, text: "Saved to OS keychain." });
      await refresh();
    } catch (e) {
      setKeyMsg({ ok: false, text: String(e) });
    } finally {
      setBusy(null);
    }
  }

  async function clearKey() {
    setBusy("key");
    setKeyMsg(null);
    try {
      await invoke("clear_api_key");
      setKeyMsg({ ok: true, text: "Cleared from keychain." });
      await refresh();
    } catch (e) {
      setKeyMsg({ ok: false, text: String(e) });
    } finally {
      setBusy(null);
    }
  }

  async function testConnection() {
    setBusy("test");
    setTestMsg(null);
    try {
      const result = await invoke<ConnectionTest>("test_connection");
      setTestMsg({
        ok: result.ok,
        text: result.ok
          ? `Connected — round-trip ${result.latency_ms}ms`
          : result.message,
      });
    } catch (e) {
      setTestMsg({ ok: false, text: String(e) });
    } finally {
      setBusy(null);
    }
  }

  async function saveHotkey() {
    setBusy("hotkey");
    setHotkeyMsg(null);
    try {
      await invoke("save_hotkey", { combo: hotkey });
      setHotkeyMsg({ ok: true, text: `Registered ${hotkey}` });
    } catch (e) {
      setHotkeyMsg({ ok: false, text: String(e) });
    } finally {
      setBusy(null);
    }
  }

  // Capture-on-keydown for the hotkey input
  useEffect(() => {
    if (!recording) return;
    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      const parts: string[] = [];
      if (e.ctrlKey || e.metaKey) parts.push("CommandOrControl");
      if (e.altKey) parts.push("Alt");
      if (e.shiftKey) parts.push("Shift");

      const k = e.key;
      if (["Control", "Alt", "Shift", "Meta", "OS", "Hyper"].includes(k)) {
        return; // wait for non-modifier
      }
      const keyName = k.length === 1 ? k.toUpperCase() : k;
      parts.push(keyName);
      setHotkey(parts.join("+"));
      setRecording(false);
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [recording]);

  if (!keyStatus) {
    return (
      <div className="pf-settings">
        <p>Loading…</p>
      </div>
    );
  }

  const keyHint = keyStatus.from_env
    ? "Currently using key from .env (env var takes precedence over keychain)"
    : keyStatus.from_keychain
      ? "Currently using key from OS keychain"
      : "No key configured — paste one below or set GROQ_API_KEY in .env";

  return (
    <div className="pf-settings">
      <h1>PromptForge Settings</h1>

      <section>
        <h2>Groq API Key</h2>
        <p className="pf-hint">{keyHint}</p>
        <p className="pf-sub">
          Get one free at{" "}
          <a href="https://console.groq.com/keys" target="_blank" rel="noreferrer">
            console.groq.com/keys
          </a>
          .
        </p>
        <div className="pf-row">
          <input
            type="password"
            placeholder="gsk_..."
            value={keyInput}
            onChange={(e) => setKeyInput(e.target.value)}
            disabled={busy === "key"}
            autoComplete="off"
            spellCheck={false}
          />
          <button onClick={saveKey} disabled={busy === "key" || !keyInput.trim()}>
            Save
          </button>
          <button
            onClick={clearKey}
            disabled={busy === "key" || !keyStatus.from_keychain}
            className="pf-secondary"
          >
            Clear
          </button>
        </div>
        {keyMsg && (
          <p className={keyMsg.ok ? "pf-msg pf-ok" : "pf-msg pf-err"}>{keyMsg.text}</p>
        )}
      </section>

      <section>
        <h2>Test Connection</h2>
        <p className="pf-hint">Pings Groq with the currently-active key.</p>
        <div className="pf-row">
          <button onClick={testConnection} disabled={busy === "test"}>
            {busy === "test" ? "Testing…" : "Test Connection"}
          </button>
        </div>
        {testMsg && (
          <p className={testMsg.ok ? "pf-msg pf-ok" : "pf-msg pf-err"}>{testMsg.text}</p>
        )}
      </section>

      <section>
        <h2>Global Hotkey</h2>
        <p className="pf-hint">
          Click the field, then press the combo you want. Save re-registers it
          system-wide.
        </p>
        <div className="pf-row">
          <input
            value={recording ? "Press a key combo…" : hotkey}
            readOnly
            onFocus={() => setRecording(true)}
            onBlur={() => setRecording(false)}
            className={recording ? "pf-recording" : ""}
            disabled={busy === "hotkey"}
          />
          <button onClick={saveHotkey} disabled={busy === "hotkey" || recording}>
            Save
          </button>
        </div>
        {hotkeyMsg && (
          <p className={hotkeyMsg.ok ? "pf-msg pf-ok" : "pf-msg pf-err"}>
            {hotkeyMsg.text}
          </p>
        )}
      </section>
    </div>
  );
}
