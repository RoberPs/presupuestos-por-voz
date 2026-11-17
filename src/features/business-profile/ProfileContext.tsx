import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { profileGet } from "../../shared/ipc/client";
import type { BusinessProfileView } from "../../shared/ipc/types";

type ProfileContextValue = {
  /** `undefined` mientras carga; `null` si aún no hay perfil (primer arranque). */
  profile: BusinessProfileView | null | undefined;
  setProfile: (p: BusinessProfileView | null) => void;
  refresh: () => Promise<BusinessProfileView | null>;
  loadError: boolean;
  /** El usuario saltó la configuración guiada en esta sesión. */
  guidedSkipped: boolean;
  skipGuided: () => void;
};

const ProfileContext = createContext<ProfileContextValue | null>(null);

/** Estado del perfil compartido entre "Mi negocio" y el aviso de perfil incompleto. */
export function ProfileProvider({ children }: { children: ReactNode }) {
  const [profile, setProfile] = useState<BusinessProfileView | null | undefined>(undefined);
  const [loadError, setLoadError] = useState(false);
  const [guidedSkipped, setGuidedSkipped] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const p = await profileGet();
      setProfile(p);
      setLoadError(false);
      return p;
    } catch {
      setLoadError(true);
      setProfile((prev) => (prev === undefined ? null : prev));
      return null;
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const value = useMemo(
    () => ({
      profile,
      setProfile,
      refresh,
      loadError,
      guidedSkipped,
      skipGuided: () => setGuidedSkipped(true),
    }),
    [profile, refresh, loadError, guidedSkipped],
  );

  return <ProfileContext.Provider value={value}>{children}</ProfileContext.Provider>;
}

export function useProfileContext() {
  const ctx = useContext(ProfileContext);
  if (!ctx) throw new Error("useProfileContext debe usarse dentro de ProfileProvider");
  return ctx;
}
