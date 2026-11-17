import { useEffect, useId, useRef, useState } from "react";
import { useForm } from "react-hook-form";
import { useNavigate, useSearchParams } from "react-router-dom";
import { Banner } from "../../shared/components/Banner";
import { Button } from "../../shared/components/Button";
import { Field } from "../../shared/components/Field";
import { useToast } from "../../shared/components/Toast";
import { UnsavedChangesGuard } from "../../shared/components/UnsavedChangesGuard";
import { es } from "../../shared/i18n/es";
import type { BusinessProfileInput, BusinessProfileView } from "../../shared/ipc/types";
import { LogoPicker } from "./LogoPicker";
import { useProfileContext } from "./ProfileContext";
import { toErrorMap, useBusinessProfile } from "./useBusinessProfile";
import styles from "./BusinessProfilePage.module.css";

export type ProfileFormValues = {
  tradeName: string;
  legalName: string;
  taxId: string;
  phone: string;
  email: string;
  website: string;
  address: { street: string; postalCode: string; city: string; province: string };
  vatRatePercent: string;
};

export function fromView(view: BusinessProfileView | null | undefined): ProfileFormValues {
  return {
    tradeName: view?.tradeName ?? "",
    legalName: view?.legalName ?? "",
    taxId: view?.taxId ?? "",
    phone: view?.phone ?? "",
    email: view?.email ?? "",
    website: view?.website ?? "",
    address: {
      street: view?.address.street ?? "",
      postalCode: view?.address.postalCode ?? "",
      city: view?.address.city ?? "",
      province: view?.address.province ?? "",
    },
    vatRatePercent: view?.vatRatePercent ?? "21",
  };
}

export function toInput(v: ProfileFormValues): BusinessProfileInput {
  return {
    tradeName: v.tradeName ?? "",
    legalName: v.legalName ?? "",
    taxId: v.taxId ?? "",
    phone: v.phone ?? "",
    email: v.email ?? "",
    website: v.website ?? "",
    address: {
      street: v.address?.street ?? "",
      postalCode: v.address?.postalCode ?? "",
      city: v.address?.city ?? "",
      province: v.address?.province ?? "",
    },
    vatRatePercent: v.vatRatePercent ?? "",
  };
}

type FieldName =
  | "tradeName"
  | "legalName"
  | "taxId"
  | "phone"
  | "email"
  | "website"
  | "address.street"
  | "address.postalCode"
  | "address.city"
  | "address.province"
  | "vatRatePercent";

export function BusinessProfilePage() {
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const guided = params.get("guiado") === "1";
  const { skipGuided } = useProfileContext();
  const { showToast } = useToast();
  const bp = useBusinessProfile();
  const { profile, loading, fieldErrors, setFieldErrors, saving, generalError, save, validate } = bp;

  const form = useForm<ProfileFormValues>({ defaultValues: fromView(null) });
  const { register, handleSubmit, reset, watch, getFieldState, getValues, formState } = form;
  const initialized = useRef(false);
  const [showAllErrors, setShowAllErrors] = useState(false);

  // Carga inicial del formulario, una sola vez, cuando llega el perfil.
  useEffect(() => {
    if (initialized.current || profile === undefined) return;
    initialized.current = true;
    reset(fromView(profile));
    if (profile) {
      setFieldErrors(toErrorMap(profile.missingFields));
      setShowAllErrors(true);
    }
  }, [profile, reset, setFieldErrors]);

  // Validación en vivo (con debounce en el hook).
  useEffect(() => {
    const sub = watch((values) => {
      if (initialized.current) validate(toInput(values as ProfileFormValues));
    });
    return () => sub.unsubscribe();
  }, [watch, validate]);

  const onSubmit = handleSubmit(async (values) => {
    setShowAllErrors(true);
    const saved = await save(toInput(values));
    if (saved) {
      reset(fromView(saved));
      showToast(es.business.saved);
      if (guided) navigate("/negocio", { replace: true });
    }
  });

  // Guardado desde el aviso de cambios sin guardar.
  const saveFromGuard = async () => {
    setShowAllErrors(true);
    const saved = await save(toInput(getValues()));
    if (!saved) return false;
    reset(fromView(saved));
    showToast(es.business.saved);
    return true;
  };

  const onSkip = () => {
    skipGuided();
    navigate("/negocio", { replace: true });
  };

  const errorFor = (name: FieldName) => {
    // Lee touchedFields para suscribirse a sus cambios.
    void formState.touchedFields;
    const visible = showAllErrors || getFieldState(name).isTouched;
    return visible ? fieldErrors[name] : undefined;
  };

  const values = watch();
  const dataId = useId();
  const addressId = useId();
  const vatId = useId();
  const previewId = useId();

  if (loading) return <p>{es.app.loading}</p>;

  return (
    <div>
      <UnsavedChangesGuard when={formState.isDirty} onSave={saveFromGuard} />
      {guided ? (
        <section className={styles.guided} aria-labelledby={`${dataId}-welcome`}>
          <h1 id={`${dataId}-welcome`}>{es.business.guided.title}</h1>
          <ol>
            {es.business.guided.steps.map((s) => (
              <li key={s}>{s}</li>
            ))}
          </ol>
          <Button variant="secondary" onClick={onSkip}>
            {es.business.guided.skip}
          </Button>
        </section>
      ) : (
        <header className={styles.header}>
          <h1>{es.business.title}</h1>
          {profile ? (
            <span
              className={`${styles.status} ${profile.status === "complete" ? styles.complete : styles.incomplete}`}
            >
              {es.business.status[profile.status]}
            </span>
          ) : null}
        </header>
      )}
      <p className={styles.intro}>{es.business.intro}</p>

      {generalError ? (
        <Banner variant="warning" role="alert">
          {generalError}
        </Banner>
      ) : null}

      <div className={styles.columns}>
        <form onSubmit={onSubmit} noValidate className={styles.form}>
          <section aria-labelledby={dataId} className={styles.card}>
            <h2 id={dataId}>{es.business.sections.data}</h2>
            <div className={styles.grid}>
              <Field label={es.business.fields.tradeName} required error={errorFor("tradeName")}>
                <input {...register("tradeName")} autoComplete="organization" />
              </Field>
              <Field label={es.business.fields.legalName} error={errorFor("legalName")}>
                <input {...register("legalName")} />
              </Field>
              <Field label={es.business.fields.taxId} required error={errorFor("taxId")}>
                <input {...register("taxId")} autoCapitalize="characters" />
              </Field>
              <Field label={es.business.fields.phone} required error={errorFor("phone")}>
                <input {...register("phone")} type="tel" inputMode="tel" autoComplete="tel" />
              </Field>
              <Field label={es.business.fields.email} error={errorFor("email")}>
                <input {...register("email")} type="email" inputMode="email" autoComplete="email" />
              </Field>
              <Field label={es.business.fields.website} error={errorFor("website")}>
                <input {...register("website")} inputMode="url" />
              </Field>
            </div>
          </section>

          <section aria-labelledby={addressId} className={styles.card}>
            <h2 id={addressId}>{es.business.sections.address}</h2>
            <div className={styles.grid}>
              <Field label={es.business.fields.street} required error={errorFor("address.street")}>
                <input {...register("address.street")} autoComplete="address-line1" />
              </Field>
              <Field
                label={es.business.fields.postalCode}
                required
                error={errorFor("address.postalCode")}
              >
                <input
                  {...register("address.postalCode")}
                  inputMode="numeric"
                  autoComplete="postal-code"
                />
              </Field>
              <Field label={es.business.fields.city} required error={errorFor("address.city")}>
                <input {...register("address.city")} autoComplete="address-level2" />
              </Field>
              <Field
                label={es.business.fields.province}
                required
                error={errorFor("address.province")}
              >
                <input {...register("address.province")} autoComplete="address-level1" />
              </Field>
            </div>
          </section>

          <section aria-labelledby={vatId} className={styles.card}>
            <h2 id={vatId}>{es.business.sections.vat}</h2>
            <Field
              label={es.business.fields.vatRatePercent}
              required
              help={es.business.vatHelp}
              suffix="%"
              error={errorFor("vatRatePercent")}
            >
              <input {...register("vatRatePercent")} inputMode="decimal" className={styles.short} />
            </Field>
          </section>

          <div className={styles.actions}>
            <Button type="submit" variant="primary" disabled={saving}>
              {saving ? es.common.saving : es.common.save}
            </Button>
          </div>
        </form>

        <aside className={styles.side}>
          <div className={styles.card}>
            <LogoPicker
              logo={profile?.logo ?? null}
              onPick={bp.pickLogo}
              onRemove={bp.removeLogo}
            />
          </div>
          <section aria-labelledby={previewId} className={`${styles.card} ${styles.preview}`}>
            <h2 id={previewId}>{es.business.sections.preview}</h2>
            <ProfilePreview values={values} logoUrl={profile?.logo?.dataUrl ?? null} />
          </section>
        </aside>
      </div>
    </div>
  );
}

function ProfilePreview({ values, logoUrl }: { values: ProfileFormValues; logoUrl: string | null }) {
  const a = values.address;
  const cityLine = [a.postalCode, a.city].filter(Boolean).join(" ");
  const placeLine = [cityLine, a.province].filter(Boolean).join(" · ");
  const lines = [
    values.legalName,
    values.taxId && `NIF/CIF: ${values.taxId}`,
    a.street,
    placeLine,
    values.phone && `Tel.: ${values.phone}`,
    values.email,
    values.website,
  ].filter(Boolean);

  if (!values.tradeName && lines.length === 0 && !logoUrl) {
    return <p className={styles.muted}>{es.business.previewEmpty}</p>;
  }
  return (
    <div className={styles.previewBody}>
      {logoUrl ? <img src={logoUrl} alt="" className={styles.previewLogo} /> : null}
      <div>
        {values.tradeName ? <p className={styles.previewName}>{values.tradeName}</p> : null}
        {lines.map((l) => (
          <p key={l} className={styles.previewLine}>
            {l}
          </p>
        ))}
      </div>
    </div>
  );
}
