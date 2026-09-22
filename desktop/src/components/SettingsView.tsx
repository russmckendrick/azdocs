import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  BookOpen,
  Check,
  ChevronRight,
  FolderOpen,
  Keyboard,
  LoaderCircle,
  Plus,
  Search,
  Trash2,
} from "lucide-react";
import type {
  AppBootstrap,
  BrandingConfig,
  Cloud,
  ConnectionCheck,
  EstateSnapshot,
  SettingsDocumentDto,
  SettingsValues,
  TenantProfile,
  ThemePreference,
} from "../types";

/** Mirrors `Cloud::ALL` in src/cloud.rs; the select renders in this order. */
const CLOUDS: Cloud[] = ["public", "usgov", "china"];
import {
  chooseBrandingPath,
  discardSettingsSecrets,
  exportSettings,
  getSettings,
  isTauri,
  loadSettingsConfig,
  migrateSettings,
  saveSettings,
  testSettings,
  openDocs,
} from "../api";
import {
  effectiveTenant,
  newTenant,
  nextTenantReference,
  removeTenant,
  renameTenant,
  splitSettingList,
} from "../settings-model";
import { errorMessage, fill } from "../format";
import { useLabels } from "../labels";
import { ViewHeading } from "./view-chrome";
import { PermissionStatus } from "./PermissionStatus";
import { SettingsConnectionDialog } from "./SettingsConnectionDialog";

type Props = {
  bootstrap: AppBootstrap;
  estate?: EstateSnapshot;
  themePreference: ThemePreference;
  resolvedTheme: "light" | "dark";
  onThemeChange: (preference: ThemePreference) => void;
  onOpenDatabase: () => void;
  onConfigChange: (bootstrap: AppBootstrap) => Promise<void>;
  onDirtyChange: (dirty: boolean) => void;
  onShowShortcuts: () => void;
  blocked: boolean;
};
const BRAND_FIELDS: Array<keyof BrandingConfig> = [
  "company",
  "title",
  "subtitle",
  "theme",
  "primary_color",
  "accent_color",
  "logo",
  "footer",
];
const ADVANCED_FIELDS: Array<keyof BrandingConfig> = [
  "page_size",
  "margin",
  "labels",
  "font_family",
  "mono_family",
  "font_dir",
];

function Field({
  label,
  detail,
  children,
}: {
  label: ReactNode;
  detail?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="settings-field">
      <div className="settings-field-name">
        {label}
        {detail && <small>{detail}</small>}
      </div>
      <div className="settings-field-control">{children}</div>
    </div>
  );
}

function ListInput({
  id,
  value,
  onChange,
  disabled,
  multiline = false,
}: {
  id: string;
  value: string[];
  onChange: (values: string[]) => void;
  disabled?: boolean;
  multiline?: boolean;
}) {
  const join = multiline ? "\n" : ", ";
  const [text, setText] = useState(value.join(join));
  const focused = useRef(false);
  const serialized = value.join(join);
  useEffect(() => {
    if (!focused.current) setText(serialized);
  }, [serialized]);
  const props = {
    id,
    disabled,
    value: text,
    onFocus: () => {
      focused.current = true;
    },
    onBlur: () => {
      focused.current = false;
      setText(value.join(join));
    },
    onChange: (
      e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
    ) => {
      setText(e.target.value);
      onChange(splitSettingList(e.target.value));
    },
  };
  return multiline ? <textarea {...props} rows={3} /> : <input {...props} />;
}

export function SettingsView(props: Props) {
  const { onDirtyChange } = props;
  const labels = useLabels();
  const {
    desktop: { settings },
    common: { access },
  } = labels;
  const words = settings.editor;
  const [document, setDocument] = useState<SettingsDocumentDto>();
  const [values, setValues] = useState<SettingsValues>();
  const [selected, setSelected] = useState("");
  const [referenceText, setReferenceText] = useState("");
  const [tab, setTab] = useState<"tenants" | "defaults" | "application">(
    "tenants",
  );
  const [search, setSearch] = useState("");
  const [tenantPanel, setTenantPanel] = useState<
    "connection" | "collection_audit" | "report_branding"
  >("connection");
  const [secretTokens, setSecretTokens] = useState<Record<string, string>>({});
  const pendingTokens = useRef<Record<string, string>>({});
  const [secrets, setSecrets] = useState<Record<string, string>>({});
  const [check, setCheck] = useState<ConnectionCheck>();
  const [testOpen, setTestOpen] = useState(false);
  const [testError, setTestError] = useState<string>();
  const [error, setError] = useState<string>();
  const [message, setMessage] = useState<string>();
  const [busy, setBusy] = useState<"save" | "test" | "load" | "migrate">();
  const [removing, setRemoving] = useState(false);
  const form = useRef<HTMLFormElement>(null);
  const testButton = useRef<HTMLButtonElement>(null);
  const scrollArea = useRef<HTMLFieldSetElement>(null);
  useEffect(() => {
    scrollArea.current?.scrollTo({ top: 0 });
  }, [tab, tenantPanel, selected]);
  const generation = useRef(0);
  const dirty = Boolean(
    values &&
    (JSON.stringify(values) !== JSON.stringify(document?.values) ||
      Object.values(secrets).some(Boolean) ||
      Object.keys(secretTokens).length > 0 ||
      referenceText !== selected),
  );
  const locked = Boolean(busy || props.blocked);
  const profile = values?.tenants[selected];
  const effective =
    values && profile ? effectiveTenant(values, selected) : undefined;

  const load = useCallback(async () => {
    const request = ++generation.current;
    const next = await getSettings();
    if (request !== generation.current) return;
    setDocument(next);
    setValues(next.values ?? undefined);
    setError(next.error ?? undefined);
    setSecrets({});
    setSecretTokens({});
    setCheck(next.check ?? undefined);
    const first =
      Object.entries(next.values?.tenants ?? {}).find(
        ([, tenant]) =>
          tenant.tenant_id.toLowerCase() ===
          props.bootstrap.activeTenantId?.toLowerCase(),
      )?.[0] ??
      next.values?.default_tenant ??
      Object.keys(next.values?.tenants ?? {})[0] ??
      "";
    setSelected(first);
    setReferenceText(first);
    setRemoving(false);
  }, [props.bootstrap.activeTenantId]);
  useEffect(() => {
    let active = true;
    const requests = generation;
    void load().catch((caught) => {
      if (active) setError(errorMessage(caught));
    });
    return () => {
      active = false;
      requests.current++;
    };
  }, [load, props.bootstrap.configPath, props.bootstrap.activeTenantId]);
  useEffect(() => {
    pendingTokens.current = secretTokens;
  }, [secretTokens]);
  useEffect(
    () => () => {
      void discardSettingsSecrets(Object.values(pendingTokens.current));
    },
    [],
  );
  useEffect(() => {
    onDirtyChange(dirty);
    return () => onDirtyChange(false);
  }, [dirty, onDirtyChange]);
  useEffect(() => {
    if (!dirty) return;
    const prevent = (event: BeforeUnloadEvent) => event.preventDefault();
    window.addEventListener("beforeunload", prevent);
    let unlisten: (() => void) | undefined;
    let active = true;
    if (isTauri)
      void import("@tauri-apps/api/window").then(
        async ({ getCurrentWindow }) => {
          const stop = await getCurrentWindow().onCloseRequested((event) => {
            event.preventDefault();
            setError(words.leave_detail);
          });
          if (active) unlisten = stop;
          else stop();
        },
      );
    return () => {
      active = false;
      unlisten?.();
      window.removeEventListener("beforeunload", prevent);
    };
  }, [dirty, words.leave_detail]);

  function edit(change: (draft: SettingsValues) => void) {
    setValues((current) => {
      if (!current) return current;
      const next = structuredClone(current);
      change(next);
      return next;
    });
    setCheck(undefined);
    setTestError(undefined);
    setMessage(undefined);
    setError(undefined);
    generation.current++;
  }
  function clearSecret(reference: string) {
    const token = secretTokens[reference];
    if (token) void discardSettingsSecrets([token]);
    setSecretTokens((old) => {
      const next = { ...old };
      delete next[reference];
      return next;
    });
    setSecrets((old) => {
      const next = { ...old };
      delete next[reference];
      return next;
    });
  }
  function editProfile(change: (draft: TenantProfile) => void) {
    if (!profile) return;
    const next = structuredClone(profile);
    change(next);
    if (
      next.tenant_id !== profile.tenant_id ||
      next.client_id !== profile.client_id ||
      next.secret_env !== profile.secret_env ||
      next.secret_ref !== profile.secret_ref
    )
      clearSecret(selected);
    edit((draft) => {
      draft.tenants[selected] = next;
    });
  }
  function chooseProfile(reference: string) {
    if (referenceText !== selected && !commitReference()) return;
    resetProfile(reference);
  }
  function resetProfile(reference: string) {
    setSelected(reference);
    setReferenceText(reference);
    setCheck(undefined);
    setTestError(undefined);
    setRemoving(false);
    setError(undefined);
  }
  function commitReference(): boolean {
    if (!values || referenceText === selected) return true;
    if (!/^[a-zA-Z0-9_-]+$/.test(referenceText)) {
      setError(words.invalid_reference);
      return false;
    }
    if (values.tenants[referenceText]) {
      setError(words.duplicate_reference);
      return false;
    }
    const renamed = renameTenant(values, selected, referenceText);
    setValues(renamed);
    setSecrets((old) => {
      const next = { ...old, [referenceText]: old[selected] ?? "" };
      delete next[selected];
      return next;
    });
    setSecretTokens((old) => {
      const next = { ...old };
      if (next[selected]) {
        next[referenceText] = next[selected];
        delete next[selected];
      }
      return next;
    });
    setSelected(referenceText);
    setError(undefined);
    return true;
  }
  async function apply(next: AppBootstrap) {
    props.onDirtyChange(false);
    await props.onConfigChange(next);
    await load();
  }
  async function save() {
    if (!values || !document || !form.current?.reportValidity()) return;
    if (referenceText !== selected) {
      commitReference();
      return;
    }
    setBusy("save");
    setError(undefined);
    const submitted = { ...secrets };
    setSecrets({});
    try {
      await apply(
        await saveSettings({
          revision: document.revision,
          values,
          secretTokens,
          newSecrets: Object.fromEntries(
            Object.entries(submitted).filter(([, secret]) => secret.length > 0),
          ),
        }),
      );
      setMessage(words.saved);
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      for (const key of Object.keys(submitted)) delete submitted[key];
      setBusy(undefined);
    }
  }
  async function test() {
    if (
      !values ||
      !profile ||
      !form.current?.reportValidity() ||
      referenceText !== selected
    )
      return;
    setBusy("test");
    setError(undefined);
    setTestError(undefined);
    setCheck(undefined);
    setTestOpen(true);
    const request = ++generation.current;
    const secret = secrets[selected] || null;
    setSecrets((old) => ({ ...old, [selected]: "" }));
    try {
      const result = await testSettings({
        reference: selected,
        values,
        newSecret: secret,
        secretToken: secretTokens[selected] ?? null,
      });
      if (request === generation.current) {
        setCheck(result.check);
        if (result.secretToken) {
          void discardSettingsSecrets(
            secretTokens[selected] ? [secretTokens[selected]] : [],
          );
          setSecretTokens((old) => ({
            ...old,
            [selected]: result.secretToken!,
          }));
        }
      } else if (result.secretToken) {
        void discardSettingsSecrets([result.secretToken]);
      }
    } catch (caught) {
      if (request === generation.current) setTestError(errorMessage(caught));
    } finally {
      setBusy(undefined);
    }
  }
  async function reload(choose: boolean) {
    if (dirty) {
      setError(words.leave_detail);
      return;
    }
    setBusy("load");
    setError(undefined);
    try {
      const next = await loadSettingsConfig(choose);
      if (next) await apply(next);
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(undefined);
    }
  }
  async function migrate() {
    setBusy("migrate");
    setError(undefined);
    try {
      await apply(await migrateSettings("default", words.new_tenant));
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(undefined);
    }
  }
  function add() {
    if (!values || (referenceText !== selected && !commitReference())) return;
    const reference = nextTenantReference(values);
    edit((draft) => {
      draft.tenants[reference] = newTenant(words.new_tenant);
      if (!draft.default_tenant) draft.default_tenant = reference;
    });
    resetProfile(reference);
    setSearch("");
    setTenantPanel("connection");
  }
  function inheritance(
    id: string,
    overridden: boolean,
    toggle: (enabled: boolean) => void,
  ) {
    return (
      <label className="settings-override" htmlFor={`override-${id}`}>
        <input
          id={`override-${id}`}
          type="checkbox"
          checked={overridden}
          onChange={(e) => toggle(e.target.checked)}
        />
        {overridden ? words.overridden : words.inherited}
      </label>
    );
  }
  function brandingFields(
    fields: Array<keyof BrandingConfig>,
    tenant: boolean,
  ) {
    if (!values) return null;
    return fields.map((key) => {
      const inherited = tenant && profile?.branding[key] == null;
      const value =
        (tenant ? effective?.branding[key] : values.branding[key]) ?? "";
      const id = `branding-${tenant ? selected : "default"}-${key}`;
      const set = (value: string) =>
        tenant
          ? editProfile((p) => {
              p.branding[key] = value;
            })
          : edit((draft) => {
              if (key === "logo" || key === "font_dir")
                draft.branding[key] = value || null;
              else draft.branding[key] = value;
            });
      return (
        <Field
          key={key}
          label={<label htmlFor={id}>{words[`field_${key}`]}</label>}
          detail={
            tenant
              ? inheritance(id, !inherited, (enabled) =>
                  editProfile((p) => {
                    if (enabled) p.branding[key] = values.branding[key] ?? "";
                    else delete p.branding[key];
                  }),
                )
              : undefined
          }
        >
          <div className="settings-input-action">
            <input
              id={id}
              value={String(value)}
              disabled={inherited}
              onChange={(e) => set(e.target.value)}
              pattern={key.endsWith("color") ? "#[a-fA-F0-9]{6}" : undefined}
              className={
                key.endsWith("color") || key === "logo" || key === "font_dir"
                  ? "mono"
                  : undefined
              }
            />
            {(key === "logo" || key === "font_dir") && (
              <button
                type="button"
                className="quiet-button"
                disabled={inherited || !isTauri}
                onClick={() =>
                  void chooseBrandingPath(key === "font_dir")
                    .then((path) => {
                      if (path) set(path);
                    })
                    .catch((caught) => setError(errorMessage(caught)))
                }
                aria-label={words.browse}
              >
                <FolderOpen size={16} />
              </button>
            )}
          </div>
        </Field>
      );
    });
  }
  function branding(tenant: boolean) {
    return (
      <section className="settings-section">
        <h2>{words.branding}</h2>
        {brandingFields(BRAND_FIELDS, tenant)}
        <details className="settings-advanced">
          <summary>{words.advanced}</summary>
          {brandingFields(ADVANCED_FIELDS, tenant)}
        </details>
      </section>
    );
  }
  function configuration(tenant: boolean) {
    if (!values) return null;
    const collection = tenant ? effective?.collect : values.collect;
    const audit = tenant ? effective?.audit : values.audit;
    return (
      <>
        <section className="settings-section">
          <h2>{words.collection}</h2>
          <Field
            label={<label htmlFor="setting-cloud">{words.field_cloud}</label>}
            detail={
              tenant
                ? inheritance("cloud", profile?.cloud != null, (enabled) =>
                    editProfile((p) => {
                      if (enabled) p.cloud = values.cloud;
                      else delete p.cloud;
                    }),
                  )
                : words.cloud_detail
            }
          >
            <select
              id="setting-cloud"
              value={
                (tenant ? (profile?.cloud ?? values.cloud) : values.cloud) ??
                "public"
              }
              disabled={tenant && profile?.cloud == null}
              onChange={(e) => {
                const cloud = e.target.value as Cloud;
                if (tenant)
                  editProfile((p) => {
                    p.cloud = cloud;
                  });
                else
                  edit((draft) => {
                    draft.cloud = cloud;
                  });
              }}
            >
              {CLOUDS.map((option) => (
                <option key={option} value={option}>
                  {settings.cloud_options[option]}
                </option>
              ))}
            </select>
          </Field>
          <Field
            label={
              <label htmlFor="setting-subscriptions">
                {words.field_subscriptions}
              </label>
            }
            detail={
              tenant
                ? inheritance(
                    "subscriptions",
                    profile?.collect.subscriptions != null,
                    (enabled) =>
                      editProfile((p) => {
                        if (enabled)
                          p.collect.subscriptions = [
                            ...values.collect.subscriptions,
                          ];
                        else delete p.collect.subscriptions;
                      }),
                  )
                : undefined
            }
          >
            <ListInput
              id="setting-subscriptions"
              multiline
              value={collection?.subscriptions ?? []}
              disabled={tenant && profile?.collect.subscriptions == null}
              onChange={(list) =>
                tenant
                  ? editProfile((p) => {
                      p.collect.subscriptions = list;
                    })
                  : edit((draft) => {
                      draft.collect.subscriptions = list;
                    })
              }
            />
            <small>{words.list_hint}</small>
          </Field>
          <Field
            label={
              <label htmlFor="setting-concurrency">
                {words.field_concurrency}
              </label>
            }
            detail={
              tenant
                ? inheritance(
                    "concurrency",
                    profile?.collect.concurrency != null,
                    (enabled) =>
                      editProfile((p) => {
                        if (enabled)
                          p.collect.concurrency = values.collect.concurrency;
                        else delete p.collect.concurrency;
                      }),
                  )
                : undefined
            }
          >
            <input
              id="setting-concurrency"
              className="settings-number"
              type="number"
              min={1}
              max={64}
              step={1}
              required
              value={collection?.concurrency ?? 4}
              disabled={tenant && profile?.collect.concurrency == null}
              onChange={(e) =>
                tenant
                  ? editProfile((p) => {
                      p.collect.concurrency = Number(e.target.value);
                    })
                  : edit((draft) => {
                      draft.collect.concurrency = Number(e.target.value);
                    })
              }
            />
          </Field>
        </section>
        <section className="settings-section">
          <h2>{words.audit}</h2>
          <Field
            label={
              <label htmlFor="setting-tags">{words.field_required_tags}</label>
            }
            detail={
              tenant
                ? inheritance(
                    "tags",
                    profile?.audit.required_tags != null,
                    (enabled) =>
                      editProfile((p) => {
                        if (enabled)
                          p.audit.required_tags = [
                            ...values.audit.required_tags,
                          ];
                        else delete p.audit.required_tags;
                      }),
                  )
                : undefined
            }
          >
            <ListInput
              id="setting-tags"
              value={audit?.required_tags ?? []}
              disabled={tenant && profile?.audit.required_tags == null}
              onChange={(list) =>
                tenant
                  ? editProfile((p) => {
                      p.audit.required_tags = list;
                    })
                  : edit((draft) => {
                      draft.audit.required_tags = list;
                    })
              }
            />
          </Field>
        </section>
      </>
    );
  }

  return (
    <div className="settings-workspace settings-editor">
      <SettingsConnectionDialog
        open={testOpen}
        testing={busy === "test"}
        tenantName={profile?.name ?? ""}
        check={check}
        error={testError}
        onClose={() => {
          setTestOpen(false);
          testButton.current?.focus();
        }}
      />
      <ViewHeading title={settings.title} description={settings.description} />
      <nav className="view-tabs" aria-label={settings.title}>
        {(["tenants", "defaults", "application"] as const).map((id) => (
          <button
            type="button"
            key={id}
            aria-current={tab === id ? "page" : undefined}
            onClick={() => setTab(id)}
          >
            {words[id]}
          </button>
        ))}
      </nav>
      {error && (
        <div className="settings-alert" role="alert">
          {error}
        </div>
      )}
      {message && (
        <div className="settings-receipt" role="status">
          <Check size={16} />
          {message}
        </div>
      )}
      {props.blocked && (
        <p className="settings-alert">{words.operation_busy}</p>
      )}
      {!document && (
        <div className="settings-loading" role="status">
          <LoaderCircle className="spin" size={20} />
          {words.reload}
        </div>
      )}
      {document?.legacy && (
        <section className="settings-migration">
          <h2>{words.legacy_title}</h2>
          <p>{words.legacy_detail}</p>
          <p className="mono">
            {document.legacyTenantId} · {document.legacyClientId}
          </p>
          <button
            type="button"
            className="collect-button"
            disabled={locked}
            onClick={() => void migrate()}
          >
            {busy === "migrate" && <LoaderCircle className="spin" size={16} />}
            {words.migrate}
          </button>
        </section>
      )}
      {document && !values && (
        <div className="settings-empty">
          <h2>{words.config_file}</h2>
          <p>{words.load_failed}</p>
          <code>{document.path}</code>
          <div className="settings-toolbar">
            <button
              type="button"
              className="quiet-button"
              onClick={() => void reload(false)}
            >
              {words.reload}
            </button>
            <button
              type="button"
              className="quiet-button"
              onClick={() => void reload(true)}
            >
              {words.load_config}
            </button>
          </div>
        </div>
      )}
      {values && (
        <form
          ref={form}
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          <fieldset
            ref={scrollArea}
            disabled={locked || document?.legacy}
            className="settings-form-fields"
          >
            {tab === "tenants" && (
              <div className="tenant-settings-layout">
                <aside
                  className="tenant-settings-list"
                  aria-label={words.tenants}
                >
                  <div className="tenant-search">
                    <Search size={15} />
                    <input
                      aria-label={words.search_tenants}
                      placeholder={words.search_tenants}
                      value={search}
                      onChange={(e) => setSearch(e.target.value)}
                    />
                  </div>
                  <div className="tenant-list-items">
                    {Object.entries(values.tenants)
                      .filter(([ref, p]) =>
                        `${ref} ${p.name} ${p.tenant_id}`
                          .toLowerCase()
                          .includes(search.toLowerCase()),
                      )
                      .map(([reference, p]) => (
                        <button
                          type="button"
                          key={reference}
                          className={selected === reference ? "selected" : ""}
                          aria-current={
                            selected === reference ? "true" : undefined
                          }
                          onClick={() => chooseProfile(reference)}
                        >
                          <span>
                            <strong>{p.name}</strong>
                            <small>{reference}</small>
                          </span>
                          {values.default_tenant === reference && (
                            <Check
                              size={14}
                              aria-label={words.default_tenant}
                            />
                          )}
                          <ChevronRight size={14} aria-hidden="true" />
                        </button>
                      ))}
                  </div>
                  <button
                    type="button"
                    className="quiet-button tenant-add"
                    onClick={add}
                  >
                    <Plus size={16} />
                    {words.add_tenant}
                  </button>
                </aside>
                <div className="tenant-settings-detail">
                  {profile ? (
                    <>
                      <div className="tenant-editor-heading">
                        <div>
                          <h2>{profile.name}</h2>
                          <p className="mono">
                            {profile.tenant_id || words.tenant_id}
                          </p>
                        </div>
                        <button
                          type="button"
                          className="quiet-button"
                          onClick={() => setRemoving(!removing)}
                          aria-label={words.remove_tenant}
                        >
                          <Trash2 size={17} />
                        </button>
                      </div>
                      {removing && (
                        <div className="settings-remove">
                          <p>{words.remove_detail}</p>
                          <button
                            type="button"
                            className="quiet-button"
                            onClick={() => {
                              setValues(removeTenant(values, selected));
                              clearSecret(selected);
                              resetProfile(
                                Object.keys(values.tenants).find(
                                  (r) => r !== selected,
                                ) ?? "",
                              );
                            }}
                          >
                            {words.confirm_remove}
                          </button>
                          <button
                            type="button"
                            className="quiet-button"
                            onClick={() => setRemoving(false)}
                          >
                            {words.cancel}
                          </button>
                        </div>
                      )}
                      <nav
                        className="tenant-panel-tabs"
                        aria-label={words.tenant_sections}
                      >
                        {(
                          [
                            "connection",
                            "collection_audit",
                            "report_branding",
                          ] as const
                        ).map((panel) => (
                          <button
                            type="button"
                            key={panel}
                            aria-current={
                              tenantPanel === panel ? "page" : undefined
                            }
                            onClick={() => setTenantPanel(panel)}
                          >
                            {words[panel]}
                          </button>
                        ))}
                      </nav>
                      {tenantPanel === "connection" && (
                        <div className="settings-connection">
                          <section className="settings-section">
                            <h2>{words.identity}</h2>
                            <div className="settings-field-pair">
                              <Field
                                label={
                                  <label htmlFor="tenant-name">
                                    {words.name}
                                  </label>
                                }
                              >
                                <input
                                  id="tenant-name"
                                  required
                                  value={profile.name}
                                  onChange={(e) =>
                                    editProfile((p) => {
                                      p.name = e.target.value;
                                    })
                                  }
                                />
                              </Field>
                              <Field
                                label={
                                  <label htmlFor="tenant-reference">
                                    {words.reference}
                                  </label>
                                }
                              >
                                <input
                                  id="tenant-reference"
                                  required
                                  pattern="[a-zA-Z0-9_-]+"
                                  value={referenceText}
                                  onChange={(e) => {
                                    setReferenceText(e.target.value);
                                    setCheck(undefined);
                                  }}
                                  onBlur={commitReference}
                                  className="mono"
                                />
                              </Field>
                            </div>
                            <Field
                              label={
                                <label htmlFor="tenant-id">
                                  {words.tenant_id}
                                </label>
                              }
                            >
                              <input
                                id="tenant-id"
                                required
                                pattern="[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}"
                                value={profile.tenant_id}
                                onChange={(e) =>
                                  editProfile((p) => {
                                    p.tenant_id = e.target.value.trim();
                                  })
                                }
                                className="mono"
                              />
                            </Field>
                            <label className="settings-check">
                              <input
                                type="checkbox"
                                checked={values.default_tenant === selected}
                                onChange={(e) =>
                                  edit((draft) => {
                                    draft.default_tenant = e.target.checked
                                      ? selected
                                      : null;
                                  })
                                }
                              />
                              {words.default_tenant}
                            </label>
                          </section>
                          <section className="settings-section">
                            <h2>{words.credentials}</h2>
                            <Field
                              label={
                                <label htmlFor="client-id">
                                  {words.client_id}
                                </label>
                              }
                            >
                              <input
                                id="client-id"
                                required
                                pattern="[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}"
                                value={profile.client_id}
                                onChange={(e) =>
                                  editProfile((p) => {
                                    p.client_id = e.target.value.trim();
                                  })
                                }
                                className="mono"
                              />
                            </Field>
                            <Field
                              label={
                                <label htmlFor="secret-source">
                                  {words.secret_source}
                                </label>
                              }
                            >
                              <select
                                id="secret-source"
                                value={
                                  profile.secret_env != null
                                    ? "environment"
                                    : "native"
                                }
                                onChange={(e) => {
                                  setSecrets((old) => ({
                                    ...old,
                                    [selected]: "",
                                  }));
                                  editProfile((p) => {
                                    if (e.target.value === "environment") {
                                      p.secret_env = "";
                                      delete p.secret_ref;
                                    } else {
                                      delete p.secret_env;
                                    }
                                  });
                                }}
                              >
                                <option value="native">
                                  {words.native_store}
                                </option>
                                <option value="environment">
                                  {words.environment}
                                </option>
                              </select>
                            </Field>
                            {profile.secret_env != null ? (
                              <Field
                                label={
                                  <label htmlFor="secret-env">
                                    {words.secret_env}
                                  </label>
                                }
                                detail={words.env_detail}
                              >
                                <input
                                  id="secret-env"
                                  required
                                  pattern="[a-zA-Z_][a-zA-Z0-9_]*"
                                  value={profile.secret_env}
                                  onChange={(e) =>
                                    editProfile((p) => {
                                      p.secret_env = e.target.value;
                                    })
                                  }
                                  className="mono"
                                />
                              </Field>
                            ) : (
                              <Field
                                label={
                                  <label htmlFor="client-secret">
                                    {words.new_secret}
                                  </label>
                                }
                              >
                                <input
                                  id="client-secret"
                                  type="password"
                                  autoComplete="new-password"
                                  value={secrets[selected] ?? ""}
                                  disabled={!isTauri}
                                  onChange={(e) => {
                                    setSecrets((old) => ({
                                      ...old,
                                      [selected]: e.target.value,
                                    }));
                                    setCheck(undefined);
                                    setMessage(undefined);
                                  }}
                                />
                                <small>
                                  {!isTauri
                                    ? words.read_only_config
                                    : secretTokens[selected]
                                      ? words.secret_pending
                                      : profile.secret_ref
                                        ? words.secret_saved
                                        : words.secret_missing}
                                </small>
                              </Field>
                            )}
                            <p className="settings-secret-note">
                              {words.secret_detail}
                            </p>
                          </section>
                          <div className="settings-connection-test">
                            <div className="settings-test-action">
                              <button
                                type="button"
                                className="quiet-button"
                                ref={testButton}
                                onClick={() => void test()}
                              >
                                {busy === "test" ? (
                                  <LoaderCircle className="spin" size={16} />
                                ) : (
                                  <Check size={16} />
                                )}
                                {busy === "test" ? words.testing : words.test}
                              </button>
                              <span>{words.test_detail}</span>
                            </div>
                            {check ? (
                              <div
                                className="settings-test-summary"
                                role="status"
                              >
                                <span>{access.verdicts[check.verdict]}</span>
                                <button
                                  type="button"
                                  onClick={() => setTestOpen(true)}
                                >
                                  {words.view_test}
                                </button>
                              </div>
                            ) : (
                              <p className="muted-copy">
                                {testError ? (
                                  <button
                                    type="button"
                                    className="settings-test-error"
                                    onClick={() => setTestOpen(true)}
                                  >
                                    {words.test_failed}
                                  </button>
                                ) : (
                                  words.not_tested
                                )}
                              </p>
                            )}
                          </div>
                        </div>
                      )}
                      {tenantPanel === "collection_audit" && (
                        <>
                          <p className="settings-intro">
                            {words.override_detail}
                          </p>
                          {configuration(true)}
                        </>
                      )}
                      {tenantPanel === "report_branding" && (
                        <>
                          <p className="settings-intro">
                            {words.override_detail}
                          </p>
                          {branding(true)}
                        </>
                      )}
                    </>
                  ) : (
                    <div className="settings-empty">
                      <h2>{words.empty_title}</h2>
                      <p>{words.empty_detail}</p>
                      <button
                        type="button"
                        className="collect-button"
                        onClick={add}
                      >
                        <Plus size={16} />
                        {words.add_tenant}
                      </button>
                    </div>
                  )}
                </div>
              </div>
            )}
            {tab === "defaults" && (
              <div className="settings-defaults">
                <p className="settings-intro">{words.default_detail}</p>
                {configuration(false)}
                {branding(false)}
              </div>
            )}
            {tab === "application" && (
              <div className="settings-defaults">
                <section className="settings-section">
                  <h2>{words.appearance}</h2>
                  <Field label={settings.theme} detail={settings.theme_detail}>
                    <div className="settings-theme-options">
                      {(["system", "light", "dark"] as const).map((option) => (
                        <button
                          type="button"
                          key={option}
                          aria-pressed={props.themePreference === option}
                          className={
                            props.themePreference === option ? "on" : ""
                          }
                          onClick={() => props.onThemeChange(option)}
                        >
                          {settings.theme_options[option]}
                        </button>
                      ))}
                    </div>
                  </Field>
                </section>
                <section className="settings-section settings-about">
                  <h2>{settings.about}</h2>
                  <p>{fill(settings.about_detail, { version: props.bootstrap.appVersion })}</p>
                  <div className="settings-toolbar">
                    <button type="button" className="quiet-button" onClick={() => void openDocs()}>
                      <BookOpen size={16} />
                      {settings.docs}
                    </button>
                    <button type="button" className="quiet-button" onClick={props.onShowShortcuts}>
                      <Keyboard size={16} />
                      {settings.shortcuts}
                    </button>
                  </div>
                </section>
                <section className="settings-section">
                  <h2>{words.database}</h2>
                  <Field
                    label={
                      <label htmlFor="database-path">
                        {words.field_db_path}
                      </label>
                    }
                    detail={words.shared_storage_detail}
                  >
                    <input
                      id="database-path"
                      required
                      className="mono"
                      value={values.storage.db_path}
                      onChange={(e) =>
                        edit((draft) => {
                          draft.storage.db_path = e.target.value;
                        })
                      }
                    />
                    <button
                      type="button"
                      className="quiet-button"
                      disabled={dirty}
                      onClick={props.onOpenDatabase}
                    >
                      {settings.open_another}
                    </button>
                  </Field>
                  <p className="settings-path mono">
                    {props.bootstrap.databasePath}
                  </p>
                </section>
                <section className="settings-section">
                  <h2>{words.config_file}</h2>
                  <p>{words.config_detail}</p>
                  <p className="settings-path mono">{document?.path}</p>
                  <div className="settings-toolbar">
                    <button
                      type="button"
                      className="quiet-button"
                      disabled={dirty}
                      onClick={() => void reload(true)}
                    >
                      <FolderOpen size={16} />
                      {words.load_config}
                    </button>
                    <button
                      type="button"
                      className="quiet-button"
                      disabled={dirty}
                      onClick={() => void reload(false)}
                    >
                      {words.reload}
                    </button>
                    <button
                      type="button"
                      className="quiet-button"
                      disabled={dirty}
                      onClick={() =>
                        void exportSettings()
                          .then((saved) => {
                            if (saved) setMessage(words.exported);
                          })
                          .catch((caught) => setError(errorMessage(caught)))
                      }
                    >
                      {words.export}
                    </button>
                  </div>
                </section>
                <details className="settings-advanced">
                  <summary>{words.effective}</summary>
                  <pre>
                    {JSON.stringify(
                      profile
                        ? effectiveTenant(values, selected)
                        : {
                            collect: values.collect,
                            audit: values.audit,
                            branding: values.branding,
                          },
                      null,
                      2,
                    )}
                  </pre>
                </details>
                {document?.check && <PermissionStatus check={document.check} />}
              </div>
            )}
          </fieldset>
          <footer className="settings-save-bar">
            <span>
              {busy === "save"
                ? words.saving
                : dirty
                  ? words.unsaved
                  : words.up_to_date}
            </span>
            <button
              type="button"
              className="quiet-button"
              disabled={locked || !dirty}
              onClick={() => {
                setValues(
                  document?.values
                    ? structuredClone(document.values)
                    : undefined,
                );
                void discardSettingsSecrets(Object.values(secretTokens));
                setSecretTokens({});
                setSecrets({});
                setError(undefined);
                setCheck(undefined);
                const first =
                  document?.values?.default_tenant ??
                  Object.keys(document?.values?.tenants ?? {})[0] ??
                  "";
                resetProfile(first);
              }}
            >
              {words.discard}
            </button>
            <button
              type="submit"
              className="collect-button"
              disabled={locked || !dirty || document?.legacy}
            >
              {busy === "save" && <LoaderCircle className="spin" size={16} />}
              {words.save}
            </button>
          </footer>
        </form>
      )}
      {!document?.legacy &&
        document?.check &&
        tab === "tenants" &&
        !dirty &&
        !check && (
          <span className="sr-only">
            {access.verdicts[document.check.verdict]}
          </span>
        )}
    </div>
  );
}
