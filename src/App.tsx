import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { fire, greet } from "./bindings";

function App() {
  const [greeting, setGreeting] = useState("");
  return (
    <div className="container">
      <h1>Welcome to Tauri!</h1>

      <div className="row">
        <a href="https://vitejs.dev" target="_blank">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://reactjs.org" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>

      <p>Click on the Tauri, Vite, and React logos to learn more.</p>

      <h1>{greeting}</h1>
      <button
        onClick={() => {
          fire("http://localhost:3000", "GET", {}, 400)
            .then((res) => {
              console.log(res);
            })
            .catch((err) => {
              console.error(err);
            });
        }}
      >
        FIRE!
      </button>
    </div>
  );
}

export default App;
