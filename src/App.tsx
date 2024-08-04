import { useEffect, useState, useSyncExternalStore } from "react";
import reactLogo from "./assets/react.svg";
import { fire, greet } from "./bindings";
import { listen } from "@tauri-apps/api/event";
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";
import Sidebar from "./views/Sidebar";
import Main from "./views/Main";

function App() {
  const metrics = useState<Response>([]);

  useEffect(() => {
    const unlisten = listen("metrics_update", (event) => {
      console.log("event", event);
    });
    return () => {
      unlisten.then((c) => c());
    };
  }, []);
  return (
    <PanelGroup autoSaveId="example" direction="horizontal">
      <Panel defaultSize={25}>
        <Sidebar />
      </Panel>
      <PanelResizeHandle />
      <Panel>
        <Main />
      </Panel>
      <PanelResizeHandle />
    </PanelGroup>
  );
}

export default App;
