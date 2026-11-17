import React from "react";
import ReactDOM from "react-dom/client";
import { RouterProvider } from "react-router-dom";
import { createAppRouter, providerFuture } from "./app/routes";
import "./shared/styles/tokens.css";
import "./shared/styles/global.css";

const router = createAppRouter();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <RouterProvider router={router} future={providerFuture} />
  </React.StrictMode>,
);
