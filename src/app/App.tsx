import { NavLink, Outlet } from "react-router-dom";
import { es } from "../shared/i18n/es";
import { ToastProvider } from "../shared/components/Toast";
import { ProfileProvider } from "../features/business-profile/ProfileContext";
import { IncompleteProfileBanner } from "./IncompleteProfileBanner";
import styles from "./App.module.css";

export function App() {
  return (
    <ProfileProvider>
      <ToastProvider>
        <div className={styles.layout}>
          <nav className={styles.nav} aria-label={es.app.navLabel}>
            <p className={styles.brand}>{es.app.name}</p>
            <ul>
              <li>
                <NavLink to="/negocio" className={({ isActive }) => (isActive ? styles.active : "")}>
                  {es.app.nav.business}
                </NavLink>
              </li>
              <li>
                <NavLink to="/tarifas" className={({ isActive }) => (isActive ? styles.active : "")}>
                  {es.app.nav.rates}
                </NavLink>
              </li>
            </ul>
          </nav>
          <main className={styles.main}>
            <IncompleteProfileBanner />
            <Outlet />
          </main>
        </div>
      </ToastProvider>
    </ProfileProvider>
  );
}
