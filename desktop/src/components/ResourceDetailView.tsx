import { useEffect, useMemo, useRef } from "react";
import {
  AlertTriangle,
  ArrowLeft,
  ChevronRight,
  CircleDot,
  GitBranch,
  ListTree,
} from "lucide-react";
import { ALL_RESOURCES_ICON } from "../azure-icons";
import type { EstateSnapshot, Resource, ResourceType } from "../types";
import { AdaptiveDataView, describeStoredValue, hasStoredValue } from "./AdaptiveDataView";

function prettyRelation(kind: string) {
  return kind.replaceAll("_", " ");
}

export function ResourceDetailView({
  resource,
  type,
  estate,
  backLabel,
  onBack,
  onSelectResource,
  onOpenTopology,
  onOpenFindings,
}: {
  resource: Resource;
  type?: ResourceType;
  estate: EstateSnapshot;
  backLabel: string;
  onBack: () => void;
  onSelectResource: (id: string) => void;
  onOpenTopology: () => void;
  onOpenFindings: () => void;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const resourceTypeMap = useMemo(
    () => new Map(estate.resourceTypes.map((item) => [item.azureType, item])),
    [estate.resourceTypes],
  );
  const subscription = estate.subscriptions.find((item) => item.id === resource.subscriptionId);
  const resourceGroup = estate.resourceGroups.find(
    (item) => item.subscriptionId === resource.subscriptionId && item.name.toLowerCase() === resource.resourceGroup,
  );
  const relatedFindings = estate.findings.filter((finding) => finding.resourceId === resource.id);
  const relatedEdges = estate.edges.filter((edge) => edge.sourceId === resource.id || edge.targetId === resource.id);
  const tags = Object.entries(resource.tags ?? {}).sort(([left], [right]) => left.localeCompare(right));
  const resourceContext = {
    resourceIdentity: {
      name: resource.name,
      azureType: resource.azureType,
      subscription: subscription?.displayName ?? "Unknown",
      subscriptionId: resource.subscriptionId,
      resourceGroup: resource.resourceGroup ?? "Subscription scope",
      groupLocation: resourceGroup?.location ?? "Not stored",
      resourceLocation: resource.location ?? "Global",
      kind: resource.kind ?? "Default",
    },
    tags: Object.fromEntries(tags),
    armIdentifiers: {
      originalDisplayId: resource.displayId,
      normalizedJoinId: resource.id,
    },
    snapshotEvidence: {
      snapshot: estate.id,
      findingCount: resource.findingCount,
      edgeCount: resource.edgeCount,
      source: "Selected SQLite snapshot; this page does not query Azure.",
    },
  };

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: 0 });
  }, [resource.id]);

  useEffect(() => {
    function handleEscape(event: KeyboardEvent) {
      if (event.key === "Escape") onBack();
    }
    window.addEventListener("keydown", handleEscape);
    return () => window.removeEventListener("keydown", handleEscape);
  }, [onBack]);

  return (
    <article className="resource-record" aria-labelledby="resource-record-title">
      <header className="resource-record-header">
        <button className="resource-record-back" onClick={onBack}>
          <ArrowLeft size={15} /> {backLabel}
        </button>
        <div className="resource-record-title">
          <img src={type?.icon ?? ALL_RESOURCES_ICON} alt="" />
          <div>
            <h1 id="resource-record-title">{resource.name}</h1>
            <p>{type?.displayName ?? resource.azureType}</p>
            <div className="resource-record-path">
              <span>{subscription?.displayName ?? resource.subscriptionId}</span>
              <ChevronRight size={12} aria-hidden="true" />
              <span>{resource.resourceGroup ?? "Subscription scope"}</span>
            </div>
          </div>
        </div>
        <div className="resource-record-header-tools">
          <dl className="resource-record-meta" aria-label="Resource summary">
            <div><dt>Location</dt><dd>{resource.location ?? "Global"}</dd></div>
            <div><dt>Kind</dt><dd>{resource.kind ?? "Default"}</dd></div>
            <div><dt>Findings</dt><dd className={relatedFindings.length > 0 ? "risk" : ""}>{relatedFindings.length}</dd></div>
            <div><dt>Relationships</dt><dd>{relatedEdges.length}</dd></div>
            <div><dt>Tags</dt><dd>{tags.length}</dd></div>
          </dl>
          <div className="resource-record-actions">
            {relatedFindings.length > 0 ? (
              <button className="quiet-button" onClick={onOpenFindings}>
                <AlertTriangle size={15} /> Review findings
              </button>
            ) : null}
            <button className="quiet-button" onClick={onOpenTopology}>
              <GitBranch size={15} /> Open topology
            </button>
          </div>
        </div>
      </header>

      <div ref={scrollRef} className="resource-record-scroll">
        <div className="resource-record-evidence">
          <section className="resource-record-section">
            <div className="resource-record-section-heading">
              <div><h2>Audit findings</h2><p>Stored checks linked to this exact resource.</p></div>
              <strong>{relatedFindings.length}</strong>
            </div>
            {relatedFindings.length > 0 ? (
              <div className="resource-record-findings">
                {relatedFindings.map((finding, index) => (
                  <article key={`${finding.queryName}-${index}`}>
                    <header>
                      <span className={`record-severity ${finding.severity}`}>{finding.severity}</span>
                      <div><h3>{finding.title}</h3><p>{finding.category} · {finding.queryName}</p></div>
                    </header>
                    <EvidenceData label="Finding evidence" value={finding.detail} compact />
                  </article>
                ))}
              </div>
            ) : (
              <div className="resource-record-clear"><CircleDot size={17} /><span>No audit findings are linked to this resource.</span></div>
            )}
          </section>

          <section className="resource-record-section resource-context-section">
            <div className="resource-record-section-heading">
              <div>
                <h2>Resource context</h2>
                <p>Identity, tags, ARM identifiers, and snapshot provenance for this stored record.</p>
              </div>
            </div>
            <AdaptiveDataView value={resourceContext} />
          </section>

          <section className="resource-record-section">
            <div className="resource-record-section-heading">
              <div><h2>Resource properties</h2><p>Stored fields and collections from Azure Resource Graph.</p></div>
              <ListTree size={18} />
            </div>
            <div className="resource-property-stack">
              <EvidenceData label="Properties" value={resource.properties} />
              <EvidenceData label="SKU" value={resource.sku} />
              <EvidenceData label="Managed identity" value={resource.identity} />
            </div>
          </section>

          <section className="resource-record-section">
            <div className="resource-record-section-heading">
              <div><h2>Relationships</h2><p>Every derived edge touching this resource, including stored edge properties.</p></div>
              <strong>{relatedEdges.length}</strong>
            </div>
            {relatedEdges.length > 0 ? (
              <div className="resource-record-relationships">
                {relatedEdges.map((edge, index) => {
                  const outbound = edge.sourceId === resource.id;
                  const otherId = outbound ? edge.targetId : edge.sourceId;
                  const other = estate.resources.find((candidate) => candidate.id === otherId);
                  const otherType = other ? resourceTypeMap.get(other.azureType) : undefined;
                  const relationshipEvidence = {
                    sourceId: edge.sourceId,
                    targetId: edge.targetId,
                    kind: edge.kind,
                    ...(hasStoredValue(edge.properties) ? { properties: edge.properties } : {}),
                  };
                  return (
                    <article key={`${edge.sourceId}-${edge.targetId}-${edge.kind}-${index}`}>
                      <button onClick={() => other && onSelectResource(other.id)} disabled={!other}>
                        <span className="relation-direction">{outbound ? "OUT" : "IN"}</span>
                        <img src={otherType?.icon ?? ALL_RESOURCES_ICON} alt="" />
                        <span><strong>{other?.name ?? otherId.split("/").at(-1)}</strong><small>{prettyRelation(edge.kind)}</small></span>
                        {other ? <ChevronRight size={14} /> : null}
                      </button>
                      <EvidenceData label="Relationship evidence" value={relationshipEvidence} compact />
                    </article>
                  );
                })}
              </div>
            ) : <p className="muted-copy">No derived relationships touch this resource.</p>}
          </section>
        </div>
      </div>
    </article>
  );
}

function EvidenceData({ label, value, compact = false }: { label: string; value: unknown; compact?: boolean }) {
  const stored = hasStoredValue(value);
  return (
    <div className={`${compact ? "adaptive-evidence compact" : "adaptive-evidence"}${stored ? "" : " empty"}`}>
      <div className="adaptive-evidence-heading">
        <span>{label}</span>
        <small>{describeStoredValue(value)}</small>
      </div>
      {stored ? <AdaptiveDataView value={value} /> : null}
    </div>
  );
}
