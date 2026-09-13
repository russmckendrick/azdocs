import type { SettingsValues, TenantProfile } from "./types";

export function splitSettingList(value: string): string[] {
  return [
    ...new Set(
      value
        .split(/[,\n]/)
        .map((item) => item.trim())
        .filter(Boolean),
    ),
  ];
}
export function newTenant(name: string): TenantProfile {
  return {
    name,
    tenant_id: "",
    client_id: "",
    collect: {},
    audit: {},
    branding: {},
  };
}
export function nextTenantReference(values: SettingsValues): string {
  let number = 1;
  while (values.tenants[`tenant-${number}`]) number++;
  return `tenant-${number}`;
}
export function renameTenant(
  values: SettingsValues,
  oldReference: string,
  reference: string,
): SettingsValues {
  if (
    !/^[a-zA-Z0-9_-]+$/.test(reference) ||
    (reference !== oldReference && values.tenants[reference])
  )
    return values;
  const next = structuredClone(values);
  next.tenants[reference] = next.tenants[oldReference];
  if (reference !== oldReference) delete next.tenants[oldReference];
  if (next.default_tenant === oldReference) next.default_tenant = reference;
  return next;
}
export function removeTenant(
  values: SettingsValues,
  reference: string,
): SettingsValues {
  const next = structuredClone(values);
  delete next.tenants[reference];
  if (next.default_tenant === reference)
    next.default_tenant = Object.keys(next.tenants).sort()[0] ?? null;
  return next;
}
function defined<T extends object>(value: T | undefined): Partial<T> {
  return Object.fromEntries(
    Object.entries(value ?? {}).filter(([, item]) => item != null),
  ) as Partial<T>;
}
export function effectiveTenant(values: SettingsValues, reference: string) {
  const tenant = values.tenants[reference];
  return {
    collect: { ...values.collect, ...defined(tenant?.collect) },
    audit: { ...values.audit, ...defined(tenant?.audit) },
    branding: { ...values.branding, ...defined(tenant?.branding) },
  };
}
