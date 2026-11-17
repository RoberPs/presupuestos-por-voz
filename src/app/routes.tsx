import { Navigate, createMemoryRouter, type RouteObject } from "react-router-dom";
import { App } from "./App";
import { BusinessProfilePage } from "../features/business-profile/BusinessProfilePage";
import { RatesPage } from "../features/rates/RatesPage";

export const routes: RouteObject[] = [
  {
    path: "/",
    element: <App />,
    children: [
      { index: true, element: <Navigate to="/negocio" replace /> },
      { path: "negocio", element: <BusinessProfilePage /> },
      { path: "tarifas", element: <RatesPage /> },
    ],
  },
];

/** Data router en memoria: `useBlocker` (aviso de cambios sin guardar) lo exige. */
export function createAppRouter(initialEntries: string[] = ["/negocio"]) {
  return createMemoryRouter(routes, { initialEntries, future: routerFuture });
}

export const routerFuture = {
  v7_fetcherPersist: true,
  v7_normalizeFormMethod: true,
  v7_partialHydration: true,
  v7_relativeSplatPath: true,
  v7_skipActionErrorRevalidation: true,
} as const;

export const providerFuture = { v7_startTransition: true } as const;
