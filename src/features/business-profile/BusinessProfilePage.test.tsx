import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { renderApp } from "../../test/renderApp";
import { logoFixture, profileFixture } from "../../test/fixtures";
import type { BusinessProfileInput, BusinessProfileView } from "../../shared/ipc/types";

type Handler = (cmd: string, args: Record<string, unknown>) => unknown;

function setupIPC(handlers: Record<string, Handler>) {
  const calls: { cmd: string; args: Record<string, unknown> }[] = [];
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    const h = handlers[cmd];
    if (h) return h(cmd, a);
    if (cmd === "profile_validate" || cmd === "rates_list" || cmd === "units_list") return [];
    return null;
  });
  return calls;
}

describe("BusinessProfilePage (US1)", () => {
  it("primer arranque sin perfil: abre el modo guiado con botón para saltarlo", async () => {
    setupIPC({ profile_get: () => null });
    renderApp("/negocio");
    expect(await screen.findByText(/Bienvenido/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Saltar por ahora" })).toBeInTheDocument();
  });

  it("al saltar la configuración muestra el aviso permanente de perfil incompleto", async () => {
    const user = userEvent.setup();
    setupIPC({ profile_get: () => null });
    renderApp("/negocio");
    await user.click(await screen.findByRole("button", { name: "Saltar por ahora" }));
    expect(await screen.findByText(/Aún no has rellenado los datos de tu negocio/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Completar ahora" })).toBeInTheDocument();
    expect(screen.queryByText(/Bienvenido/)).not.toBeInTheDocument();
  });

  it("muestra junto a cada campo los errores que devuelve profile_validate", async () => {
    const user = userEvent.setup();
    setupIPC({
      profile_get: () => null,
      profile_validate: () => [
        {
          field: "taxId",
          code: "TAX_ID_INVALID",
          message: "El NIF, NIE o CIF no es válido. Revisa los números y la letra.",
        },
      ],
    });
    renderApp("/negocio");
    const taxId = await screen.findByLabelText(/NIF \/ CIF/);
    await user.type(taxId, "12345678A");
    await user.tab();
    await waitFor(() =>
      expect(taxId).toHaveAccessibleDescription(/El NIF, NIE o CIF no es válido/),
    );
    expect(taxId).toHaveAttribute("aria-invalid", "true");
  });

  it("guarda la dirección en cuatro campos y confirma con un aviso", async () => {
    const user = userEvent.setup();
    let saved: BusinessProfileInput | null = null;
    setupIPC({
      profile_get: () => null,
      profile_save: (_c, args) => {
        saved = args.input as BusinessProfileInput;
        return profileFixture({ tradeName: saved.tradeName });
      },
    });
    renderApp("/negocio");
    await user.type(await screen.findByLabelText(/Nombre comercial/), "Fontanería López");
    await user.type(screen.getByLabelText(/NIF \/ CIF/), "12345678Z");
    await user.type(screen.getByLabelText(/Teléfono/), "600 123 456");
    await user.type(screen.getByLabelText(/Calle y número/), "Calle Mayor 1");
    await user.type(screen.getByLabelText(/Código postal/), "28001");
    await user.type(screen.getByLabelText(/Localidad/), "Madrid");
    await user.type(screen.getByLabelText(/Provincia/), "Madrid");
    await user.click(screen.getByRole("button", { name: "Guardar" }));

    await waitFor(() => expect(saved).not.toBeNull());
    expect(saved!.address).toEqual({
      street: "Calle Mayor 1",
      postalCode: "28001",
      city: "Madrid",
      province: "Madrid",
    });
    expect(saved!.vatRatePercent).toBe("21");
    expect(await screen.findByText("Datos guardados")).toBeInTheDocument();
  });

  it("si el logo se rechaza muestra el motivo y conserva la vista previa anterior", async () => {
    const user = userEvent.setup();
    const profile: BusinessProfileView = profileFixture({ logo: logoFixture });
    setupIPC({
      profile_get: () => profile,
      logo_pick_and_set: () => {
        throw {
          kind: "logoRejected",
          code: "LOGO_TOO_LARGE",
          message: "La imagen pesa más de 5 MB. Elige una imagen más pequeña.",
        };
      },
    });
    renderApp("/negocio");
    const logoSection = await screen.findByRole("region", { name: "Logo" });
    expect(within(logoSection).getByRole("img", { name: "Logo del negocio" })).toHaveAttribute(
      "src",
      logoFixture.dataUrl,
    );
    await user.click(within(logoSection).getByRole("button", { name: "Cambiar logo" }));
    expect(await within(logoSection).findByRole("alert")).toHaveTextContent(/más de 5 MB/);
    expect(within(logoSection).getByRole("img", { name: "Logo del negocio" })).toHaveAttribute(
      "src",
      logoFixture.dataUrl,
    );
  });

  describe("US3: modificar datos y logo", () => {
    async function editPhoneAndLeave(user: ReturnType<typeof userEvent.setup>) {
      const phone = await screen.findByLabelText(/Teléfono/);
      await user.clear(phone);
      await user.type(phone, "911 222 333");
      await user.click(screen.getByRole("link", { name: "Tarifas" }));
      return screen.findByRole("dialog", { name: "Tienes cambios sin guardar" });
    }

    it("con cambios sin guardar avisa al salir; Cancelar se queda en la página", async () => {
      const user = userEvent.setup();
      setupIPC({ profile_get: () => profileFixture() });
      const { router } = renderApp("/negocio");
      const dialog = await editPhoneAndLeave(user);
      await user.click(within(dialog).getByRole("button", { name: "Cancelar" }));
      expect(router.state.location.pathname).toBe("/negocio");
      expect(screen.getByLabelText(/Teléfono/)).toHaveValue("911 222 333");
    });

    it("Descartar navega sin guardar", async () => {
      const user = userEvent.setup();
      const calls = setupIPC({ profile_get: () => profileFixture() });
      const { router } = renderApp("/negocio");
      const dialog = await editPhoneAndLeave(user);
      await user.click(within(dialog).getByRole("button", { name: "Descartar" }));
      await waitFor(() => expect(router.state.location.pathname).toBe("/tarifas"));
      expect(calls.some((c) => c.cmd === "profile_save")).toBe(false);
    });

    it("Guardar llama a profile_save y después navega", async () => {
      const user = userEvent.setup();
      const calls = setupIPC({
        profile_get: () => profileFixture(),
        profile_save: (_c, args) =>
          profileFixture({ phone: (args.input as BusinessProfileInput).phone }),
      });
      const { router } = renderApp("/negocio");
      const dialog = await editPhoneAndLeave(user);
      await user.click(within(dialog).getByRole("button", { name: "Guardar" }));
      await waitFor(() => expect(router.state.location.pathname).toBe("/tarifas"));
      const save = calls.find((c) => c.cmd === "profile_save");
      expect((save!.args.input as BusinessProfileInput).phone).toBe("911 222 333");
    });

    it("Quitar logo pide confirmación y llama a logo_remove", async () => {
      const user = userEvent.setup();
      const calls = setupIPC({
        profile_get: () => profileFixture({ logo: logoFixture }),
        logo_remove: () => profileFixture({ logo: null }),
      });
      renderApp("/negocio");
      const logoSection = await screen.findByRole("region", { name: "Logo" });
      await user.click(within(logoSection).getByRole("button", { name: "Quitar logo" }));
      const dialog = await screen.findByRole("dialog", { name: "¿Quitar el logo?" });
      expect(calls.some((c) => c.cmd === "logo_remove")).toBe(false);
      await user.click(within(dialog).getByRole("button", { name: "Quitar logo" }));
      await waitFor(() => expect(calls.some((c) => c.cmd === "logo_remove")).toBe(true));
      expect(await within(logoSection).findByText("Sin logo")).toBeInTheDocument();
    });
  });

  it("perfil incompleto: muestra el aviso con el número de datos que faltan", async () => {
    setupIPC({
      profile_get: () =>
        profileFixture({
          status: "incomplete",
          phone: "",
          missingFields: [{ field: "phone", code: "REQUIRED", message: "Este dato es obligatorio." }],
        }),
    });
    renderApp("/tarifas");
    expect(await screen.findByText("Tu perfil está incompleto: falta 1 dato")).toBeInTheDocument();
  });
});
