import { WebsiteScreenshots } from "./WebsiteScreenshots";
import { useEffect, useRef, useState } from "react";
import {
  AlertTriangle,
  ArrowLeft,
  ChevronRight,
  CircleDot,
  ClipboardCopy,
  GitBranch,
  ListTree,
} from "lucide-react";
import { resourceIcon } from "../azure-icons";
import { displayKind, displayLocation } from "../azure-values";
import type { EstateSnapshot, Resource, ResourceDetail, ResourceType } from "../types";
import { copyText, getResourceDetail } from "../api";
import { AdaptiveDataView } from "./AdaptiveDataView";
import { describeStoredValue, hasStoredValue } from "./stored-values";
import { errorMessage, plural, resourceName, spaced } from "../format";
import { useLabels } from "../labels";
import { EmptyState } from "./view-chrome";
import { useEscapeKey, useResourceTypeMap } from "../estate-lookups";

/** The connector text for an edge kind: from the labels, spaced key as the fallback. */
function prettyRelation(kind: string, edgeKinds: Record<string, string>) {
  return edgeKinds[kind] ?? spaced(kind);
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
  const resourceTypeMap = useResourceTypeMap(estate);
  // The estate rows carry only flags for the heavy bags; the record reads
  // them from SQLite when it opens, so a large snapshot loads without them.
  const [detail, setDetail] = useState<ResourceDetail | null>();
  const [detailError, setDetailError] = useState<string>();
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    setCopied(false);
  }, [resource.id]);
  useEffect(() => {
    let active = true;
    setDetail(undefined);
    setDetailError(undefined);
    if (!resource.hasProperties && !resource.hasSku && !resource.hasIdentity) {
      setDetail(null);
      return;
    }
    getResourceDetail(estate.id, resource.id)
      .then((next) => {
        if (active) setDetail(next);
      })
      .catch((caught) => {
        if (active) setDetailError(errorMessage(caught));
      });
    return () => {
      active = false;
    };
  }, [estate.id, resource.id, resource.hasProperties, resource.hasSku, resource.hasIdentity]);
  const { common, desktop: { record: words, topology: { edge_kinds: edgeKinds } } } = useLabels();
  const subscription = estate.subscriptions.find((item) => item.id === resource.subscriptionId);
  const resourceGroup = estate.resourceGroups.find(
    (item) => item.subscriptionId === resource.subscriptionId && item.name.toLowerCase() === resource.resourceGroup,
  );
  const relatedFindings = estate.findings.filter((finding) => finding.resourceId === resource.id);
  const relatedEdges = estate.edges.filter((edge) => edge.sourceId === resource.id || edge.targetId === resource.id);
  const tags = Object.entries(resource.tags ?? {}).sort(([left], [right]) => left.localeCompare(right));
  const locationName = displayLocation(estate.azureMetadata, resource.location);
  const groupLocationName = displayLocation(estate.azureMetadata, resourceGroup?.location, common.columns.location);
  const kindName = displayKind(estate.azureMetadata, resource.azureType, resource.kind);
  const resourceContext = {
    resourceIdentity: {
      name: resource.name,
      azureType: resource.azureType,
      subscription: subscription?.displayName ?? words.unknown_subscription,
      subscriptionId: resource.subscriptionId,
      resourceGroup: resource.resourceGroup ?? common.subscription_scope,
      groupLocation: groupLocationName,
      resourceLocation: locationName,
      kind: kindName,
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
      source: words.source_note,
    },
  };

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: 0 });
  }, [resource.id]);

  // The record is always open while mounted.
  useEscapeKey(true, onBack);

  return (
    <article className="resource-record" aria-labelledby="resource-record-title">
      <header className="resource-record-header">
        <button className="resource-record-back" onClick={onBack}>
          <ArrowLeft size={15} /> {backLabel}
        </button>
        <div className="resource-record-title">
          <img src={resourceIcon(type)} alt="" />
          <div>
            <h1 id="resource-record-title">{resource.name}</h1>
            <p>{type?.displayName ?? resource.azureType}</p>
            <div className="resource-record-path">
              <span>{subscription?.displayName ?? resource.subscriptionId}</span>
              <ChevronRight size={12} aria-hidden="true" />
              <span>{resource.resourceGroup ?? common.subscription_scope}</span>
            </div>
          </div>
        </div>
        <div className="resource-record-header-tools">
          <dl className="resource-record-meta" aria-label={words.summary_aria}>
            <div><dt>{common.columns.location}</dt><dd>{locationName}</dd></div>
            <div><dt>{common.columns.kind}</dt><dd>{kindName}</dd></div>
            <div><dt>{common.columns.findings}</dt><dd className={relatedFindings.length > 0 ? "risk" : ""}>{relatedFindings.length}</dd></div>
            <div><dt>{words.relationships}</dt><dd>{relatedEdges.length}</dd></div>
            <div><dt>{common.columns.tags}</dt><dd>{tags.length}</dd></div>
          </dl>
          <div className="resource-record-actions">
            <button
              className="quiet-button"
              onClick={() => {
                copyText(resource.displayId)
                  .then(() => setCopied(true))
                  .catch(() => setCopied(false));
              }}
              title={resource.displayId}
            >
              <ClipboardCopy size={15} /> {copied ? words.copied : words.copy_id}
            </button>
            {relatedFindings.length > 0 ? (
              <button className="quiet-button" onClick={onOpenFindings}>
                <AlertTriangle size={15} /> {words.review_findings}
              </button>
            ) : null}
            <button className="quiet-button" onClick={onOpenTopology} disabled={relatedEdges.length === 0}>
              <GitBranch size={15} /> {relatedEdges.length > 0
                ? plural(words.explore_relationships, relatedEdges.length)
                : words.no_relationships}
            </button>
          </div>
        </div>
      </header>

      <div ref={scrollRef} className="resource-record-scroll">
        <div className="resource-record-evidence">
          <WebsiteScreenshots resourceId={resource.id} snapshotId={estate.id} />
          <section className="resource-record-section">
            <div className="resource-record-section-heading">
              <div><h2>{words.findings_title}</h2><p>{words.findings_detail}</p></div>
              <strong>{relatedFindings.length}</strong>
            </div>
            {relatedFindings.length > 0 ? (
              <div className="resource-record-findings">
                {relatedFindings.map((finding, index) => (
                  <article key={`${finding.queryName}-${index}`}>
                    <header>
                      <span className={`record-severity ${finding.severity}`}>{common.severity[finding.severity].name}</span>
                      <div><h3>{finding.title}</h3><p>{finding.category} · {finding.queryName}</p></div>
                    </header>
                    <EvidenceData label={words.finding_evidence} value={finding.detail} compact />
                  </article>
                ))}
              </div>
            ) : (
              <EmptyState
                className="resource-record-clear"
                icon={<CircleDot size={17} />}
                detail={words.no_findings}
              />
            )}
          </section>

          <section className="resource-record-section resource-context-section">
            <div className="resource-record-section-heading">
              <div>
                <h2>{words.context_title}</h2>
                <p>{words.context_detail}</p>
              </div>
            </div>
            <AdaptiveDataView value={resourceContext} />
          </section>

          <section className="resource-record-section">
            <div className="resource-record-section-heading">
              <div><h2>{words.properties_title}</h2><p>{words.properties_detail}</p></div>
              <ListTree size={18} />
            </div>
            {detailError ? <p className="muted-copy risk">{detailError}</p> : null}
            {detail === undefined && !detailError ? (
              <p className="muted-copy" role="status">{words.loading_properties}</p>
            ) : (
              <div className="resource-property-stack">
                <EvidenceData label={words.properties} value={detail?.properties} />
                <EvidenceData label={words.sku} value={detail?.sku} />
                <EvidenceData label={words.identity} value={detail?.identity} />
              </div>
            )}
          </section>

          <section className="resource-record-section">
            <div className="resource-record-section-heading">
              <div><h2>{words.relationships_title}</h2><p>{words.relationships_detail}</p></div>
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
                        <span className="relation-direction">{outbound ? words.outbound : words.inbound}</span>
                        <img src={resourceIcon(otherType)} alt="" />
                        <span><strong>{other?.name ?? resourceName(otherId)}</strong><small>{prettyRelation(edge.kind, edgeKinds)}</small></span>
                        {other ? <ChevronRight size={14} /> : null}
                      </button>
                      <EvidenceData label={words.relationship_evidence} value={relationshipEvidence} compact />
                    </article>
                  );
                })}
              </div>
            ) : <p className="muted-copy">{words.no_relationship_rows}</p>}
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
