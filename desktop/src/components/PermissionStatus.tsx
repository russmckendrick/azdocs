import { AlertTriangle, ShieldCheck } from "lucide-react";
import type { ConnectionCheck } from "../types";
import { useLabels } from "../labels";
import { dayMonthTime, fill } from "../format";

export function PermissionStatus({
  check,
  expanded = false,
}: {
  check: ConnectionCheck;
  expanded?: boolean;
}) {
  const {
    common: { access },
    desktop: {
      settings: { editor: words },
    },
  } = useLabels();
  const safe = check.verdict === "read_only";
  return (
    <section
      className={`permission-status ${safe ? "permission-good" : "permission-warning"}`}
      aria-label={access.title}
    >
      <div className="permission-heading">
        {safe ? (
          <ShieldCheck size={19} aria-hidden="true" />
        ) : (
          <AlertTriangle size={19} aria-hidden="true" />
        )}
        <strong>{access.verdicts[check.verdict]}</strong>
        <span>
          {fill(access.checked, { time: dayMonthTime(check.checkedAt) })}
        </span>
      </div>
      <p>{access.detail}</p>
      {check.issues.length > 0 && (
        <ul>
          {check.issues.map((issue, i) => (
            <li key={`${issue.scope}-${i}`}>
              {access.reasons[issue.kind]}{" "}
              {issue.scope && <span className="mono">{issue.scope}</span>}
            </li>
          ))}
        </ul>
      )}
      <details open={expanded}>
        <summary>
          {words.permission_roles} · {check.grants.length}
        </summary>
        <div className="settings-evidence-scroll">
          <table>
            <thead>
              <tr>
                <th>{words.permission_role}</th>
                <th>{words.scope}</th>
                <th>{words.status}</th>
              </tr>
            </thead>
            <tbody>
              {check.grants.map((grant, i) => (
                <tr key={`${grant.roleId}-${i}`}>
                  <td>{grant.role}</td>
                  <td className="mono">{grant.scope}</td>
                  <td>{access.verdicts[grant.verdict]}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </details>
      <details open={expanded}>
        <summary>
          {words.visible_subscriptions} · {check.subscriptions.length}
        </summary>
        {check.subscriptions.length ? (
          <ul>
            {check.subscriptions.map((sub) => (
              <li key={sub.id}>
                {sub.name} <span className="mono">{sub.id}</span>
              </li>
            ))}
          </ul>
        ) : (
          <p>{words.no_subscriptions}</p>
        )}
      </details>
    </section>
  );
}
