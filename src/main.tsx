import '@/index.css';
import { Overlay } from "@/windows/Overlay";
import { Settings } from "@/windows/Settings";
import React from "react";
import ReactDOM from "react-dom/client";
import { Toaster } from "sonner";

const params = new URLSearchParams(window.location.search);
const windowName = params.get("window");

document.documentElement.setAttribute("data-window", windowName || "settings");
document.body.setAttribute("data-window", windowName || "settings");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {windowName === "overlay" ? <Overlay /> : <Settings />}
    <Toaster richColors position="bottom-center" />
  </React.StrictMode>,
);
