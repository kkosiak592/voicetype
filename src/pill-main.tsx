import React from "react";
import ReactDOM from "react-dom/client";
import { Pill } from "./Pill";
import "./pill.css";

ReactDOM.createRoot(document.getElementById("pill-root") as HTMLElement).render(
  <React.StrictMode>
    <Pill />
  </React.StrictMode>
);
