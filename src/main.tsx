import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// Apply default theme attributes before first render
document.documentElement.setAttribute("data-theme", "dark");
document.documentElement.setAttribute("data-density", "comfortable");
document.documentElement.setAttribute("data-font-size", "medium");
document.documentElement.classList.add("dark");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
