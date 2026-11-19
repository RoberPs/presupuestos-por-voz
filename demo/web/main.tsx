import React from "react";
import ReactDOM from "react-dom/client";
import { RouterProvider } from "react-router-dom";
import { createAppRouter, providerFuture } from "../../src/app/routes";
import "../../src/shared/styles/tokens.css";
import "../../src/shared/styles/global.css";
import { DemoBanner } from "./DemoBanner";
import { startDemoBackend } from "./demoBackend";

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);

startDemoBackend()
  .then(() => {
    // La demo ya trae el negocio y las tarifas: empieza en el presupuesto.
    const router = createAppRouter(["/presupuesto"]);
    root.render(
      <React.StrictMode>
        <DemoBanner />
        <RouterProvider router={router} future={providerFuture} />
      </React.StrictMode>,
    );
  })
  .catch(() => {
    root.render(
      <p style={{ padding: 24 }}>
        Este navegador no puede ejecutar la demo (necesita WebAssembly). Prueba con una versión
        reciente de Chrome, Edge, Firefox o Safari.
      </p>,
    );
  });
