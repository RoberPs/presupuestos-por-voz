import { useEffect, useRef } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { Banner } from "../shared/components/Banner";
import { Button } from "../shared/components/Button";
import { es } from "../shared/i18n/es";
import { useProfileContext } from "../features/business-profile/ProfileContext";

/**
 * Primer arranque sin perfil → configuración guiada (FR-001c).
 * Perfil inexistente o incompleto → aviso permanente en todas las pantallas (FR-008).
 */
export function IncompleteProfileBanner() {
  const { profile, guidedSkipped } = useProfileContext();
  const navigate = useNavigate();
  const location = useLocation();
  const redirected = useRef(false);
  const onGuided =
    location.pathname === "/negocio" && new URLSearchParams(location.search).get("guiado") === "1";

  useEffect(() => {
    if (redirected.current || profile === undefined) return;
    redirected.current = true;
    if (profile === null && !guidedSkipped && !onGuided) {
      navigate("/negocio?guiado=1", { replace: true });
    }
  }, [profile, guidedSkipped, onGuided, navigate]);

  if (profile === undefined || onGuided) return null;
  if (profile !== null && profile.status === "complete") return null;

  const text =
    profile === null
      ? es.incompleteBanner.noProfile
      : es.incompleteBanner.message(profile.missingFields.length);

  return (
    <Banner
      variant="warning"
      action={
        <Button variant="secondary" onClick={() => navigate("/negocio")}>
          {es.incompleteBanner.action}
        </Button>
      }
    >
      {text}
    </Banner>
  );
}
