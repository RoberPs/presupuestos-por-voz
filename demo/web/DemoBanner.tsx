import styles from "./DemoBanner.module.css";

const REPO = "https://github.com/RoberPs/presupuestos-por-voz";
const DOWNLOAD = `${REPO}/releases/tag/v0.1.0`;

/** Aviso fijo de la demo: qué es, qué no incluye y cómo probarla. */
export function DemoBanner() {
  return (
    <aside className={styles.banner} aria-label="Sobre esta demo">
      <p className={styles.text}>
        <strong>Demo web.</strong> Es la misma interfaz y el mismo núcleo Rust que la app de
        escritorio, compilado a WebAssembly. Los datos son de ejemplo y se borran al cerrar la
        pestaña; el dictado por voz solo está en la app de Windows.{" "}
        <span className={styles.hint}>
          Prueba: en «Presupuesto», pulsa «Escribir la visita» y genera el presupuesto.
        </span>
      </p>
      <nav className={styles.links} aria-label="Enlaces de la demo">
        <a href={DOWNLOAD}>Descargar para Windows</a>
        <a href={REPO}>Código en GitHub</a>
        <button type="button" onClick={() => window.location.reload()}>
          Empezar de nuevo
        </button>
      </nav>
    </aside>
  );
}
