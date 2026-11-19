import type { BusinessProfileView, LogoView } from "../shared/ipc/types";

export const logoFixture: LogoView = {
  dataUrl: "data:image/png;base64,AAAA",
  mime: "image/png",
  widthPx: 400,
  heightPx: 300,
  lowResolution: false,
};

export function profileFixture(overrides: Partial<BusinessProfileView> = {}): BusinessProfileView {
  return {
    tradeName: "Fontanería López",
    legalName: null,
    taxId: "12345678Z",
    phone: "600 123 456",
    email: null,
    website: null,
    address: { street: "Calle Mayor 1", postalCode: "28001", city: "Madrid", province: "Madrid" },
    vatRatePercent: "21,00",
    currency: "EUR",
    logo: null,
    status: "complete",
    missingFields: [],
    ...overrides,
  };
}
