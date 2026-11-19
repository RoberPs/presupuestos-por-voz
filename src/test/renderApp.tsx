import { render } from "@testing-library/react";
import { RouterProvider } from "react-router-dom";
import { createAppRouter, providerFuture } from "../app/routes";

/** Renderiza la app completa en la ruta indicada (para pruebas con mockIPC). */
export function renderApp(path = "/negocio") {
  const router = createAppRouter([path]);
  const utils = render(<RouterProvider router={router} future={providerFuture} />);
  return { router, ...utils };
}
