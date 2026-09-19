import type {
  AppBootstrap,
  Edge,
  ExportRequest,
  EstateSnapshot,
  Finding,
  QueryDefMeta,
  QueryRows,
  Resource,
  ResourceType,
  Severity,
} from "./types";
import { buildResourceGroupTopology } from "./components/topology-model";
import { DEFAULT_LABELS } from "./labels";

const ids = {
  vnetHub:
    "/subscriptions/sub-prod/resourcegroups/rg-network/providers/microsoft.network/virtualnetworks/vnet-hub",
  vnetApp:
    "/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.network/virtualnetworks/vnet-app",
  nsg: "/subscriptions/sub-prod/resourcegroups/rg-network/providers/microsoft.network/networksecuritygroups/nsg-app",
  nsgUnused:
    "/subscriptions/sub-prod/resourcegroups/rg-network/providers/microsoft.network/networksecuritygroups/nsg-unused",
  vm: "/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.compute/virtualmachines/vm-app-01",
  disk: "/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.compute/disks/vm-app-01-os",
  nic: "/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.network/networkinterfaces/vm-app-01-nic",
  pip: "/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.network/publicipaddresses/vm-app-01-pip",
  storage:
    "/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.storage/storageaccounts/stprodapp01",
  privateEndpoint:
    "/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.network/privateendpoints/pe-sql",
  sql: "/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.sql/servers/sql-prod",
  logs: "/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.operationalinsights/workspaces/law-prod",
  hostPool:
    "/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.desktopvirtualization/hostpools/hp-prod",
  web: "/subscriptions/sub-dev/resourcegroups/rg-dev/providers/microsoft.web/sites/web-dev",
  plan: "/subscriptions/sub-dev/resourcegroups/rg-dev/providers/microsoft.web/serverfarms/asp-dev",
};

const typeNames: Record<string, string> = {
  "microsoft.network/virtualnetworks": "Virtual Network",
  "microsoft.network/networksecuritygroups": "Network Security Group",
  "microsoft.compute/virtualmachines": "Virtual Machine",
  "microsoft.compute/disks": "Managed Disk",
  "microsoft.network/networkinterfaces": "Network Interface",
  "microsoft.network/publicipaddresses": "Public IP Address",
  "microsoft.storage/storageaccounts": "Storage Account",
  "microsoft.network/privateendpoints": "Private Endpoint",
  "microsoft.sql/servers": "SQL Server",
  "microsoft.operationalinsights/workspaces": "Log Analytics Workspace",
  "microsoft.desktopvirtualization/hostpools": "AVD Host Pool",
  "microsoft.web/sites": "App Service",
  "microsoft.web/serverfarms": "App Service Plan",
};

const typeColors: Record<string, string> = {
  "microsoft.network": "#159455",
  "microsoft.compute": "#1874d1",
  "microsoft.storage": "#c38b00",
  "microsoft.sql": "#a34bb7",
  "microsoft.operationalinsights": "#0f858d",
  "microsoft.desktopvirtualization": "#6554c0",
  "microsoft.web": "#dc5a1f",
};

const typeIcons: Record<string, string> = {
  "microsoft.compute/disks": "/icons/compute/10032-icon-service-Disks.svg",
  "microsoft.compute/virtualmachines":
    "/icons/compute/10021-icon-service-Virtual-Machine.svg",
  "microsoft.desktopvirtualization/hostpools":
    "/icons/compute/00328-icon-service-Host-Pools.svg",
  "microsoft.network/networkinterfaces":
    "/icons/networking/10080-icon-service-Network-Interfaces.svg",
  "microsoft.network/networksecuritygroups":
    "/icons/networking/10067-icon-service-Network-Security-Groups.svg",
  "microsoft.network/privateendpoints":
    "/icons/other/02579-icon-service-Private-Endpoints.svg",
  "microsoft.network/publicipaddresses":
    "/icons/networking/10069-icon-service-Public-IP-Addresses.svg",
  "microsoft.network/virtualnetworks":
    "/icons/networking/10061-icon-service-Virtual-Networks.svg",
  "microsoft.operationalinsights/workspaces":
    "/icons/monitor/00009-icon-service-Log-Analytics-Workspaces.svg",
  "microsoft.sql/servers": "/icons/databases/10132-icon-service-SQL-Server.svg",
  "microsoft.storage/storageaccounts":
    "/icons/storage/10086-icon-service-Storage-Accounts.svg",
  "microsoft.web/serverfarms":
    "/icons/app services/00046-icon-service-App-Service-Plans.svg",
  "microsoft.web/sites":
    "/icons/app services/10035-icon-service-App-Services.svg",
};

function typeColor(azureType: string) {
  const prefix = Object.keys(typeColors).find((key) =>
    azureType.startsWith(key),
  );
  return prefix ? typeColors[prefix] : "#526273";
}

function iconData(azureType: string) {
  return (
    typeIcons[azureType] ??
    "/icons/general/10001-icon-service-All-Resources.svg"
  );
}

function resource(
  id: string,
  name: string,
  azureType: string,
  resourceGroup: string,
  subscriptionId: string,
  options: Partial<Resource> = {},
): Resource {
  return {
    id,
    displayId: id,
    name,
    azureType,
    resourceGroup,
    subscriptionId,
    location: subscriptionId === "sub-dev" ? "ukwest" : "uksouth",
    tags:
      subscriptionId === "sub-prod"
        ? { env: "prod", owner: "platform" }
        : undefined,
    findingCount: 0,
    edgeCount: 0,
    properties: {},
    ...options,
  };
}

const findings: Finding[] = [
  {
    queryName: "storage_public_blob_access",
    category: "storage",
    severity: "high",
    resourceId: ids.storage,
    title: "Public blob access is enabled",
    detail: { summary: "stprodapp01 allows anonymous public blob access." },
  },
  {
    queryName: "nsg_open_to_internet",
    category: "networking",
    severity: "high",
    resourceId: ids.nsg,
    title: "SSH is open to the Internet",
    detail: { ruleName: "allow-ssh", port: "22", priority: 100 },
  },
  {
    queryName: "web_app_https_only_disabled",
    category: "app services",
    severity: "medium",
    resourceId: ids.web,
    title: "HTTPS-only traffic is not enforced",
    detail: { summary: "web-dev accepts unencrypted HTTP traffic." },
  },
  {
    queryName: "unassociated_nsgs",
    category: "networking",
    severity: "low",
    resourceId: ids.nsgUnused,
    title: "Network security group is not associated",
    detail: { summary: "nsg-unused has no subnet or NIC association." },
  },
];

const edges: Edge[] = [
  { sourceId: ids.vnetHub, targetId: ids.vnetApp, kind: "peered_with" },
  { sourceId: ids.nsg, targetId: ids.vnetHub, kind: "nsg_attached" },
  { sourceId: ids.nic, targetId: ids.vm, kind: "attached_to" },
  { sourceId: ids.disk, targetId: ids.vm, kind: "attached_to" },
  { sourceId: ids.nic, targetId: ids.vnetApp, kind: "nic_in_subnet" },
  { sourceId: ids.pip, targetId: ids.nic, kind: "attached_to" },
  {
    sourceId: ids.privateEndpoint,
    targetId: ids.sql,
    kind: "private_endpoint_for",
  },
  { sourceId: ids.privateEndpoint, targetId: ids.vnetApp, kind: "in_vnet" },
  { sourceId: ids.web, targetId: ids.plan, kind: "depends_on" },
];

const resources: Resource[] = [
  resource(
    ids.vnetHub,
    "vnet-hub",
    "microsoft.network/virtualnetworks",
    "rg-network",
    "sub-prod",
    {
      properties: {
        addressSpace: { addressPrefixes: ["10.0.0.0/16"] },
        subnetCount: 2,
        peeringState: "Connected",
      },
    },
  ),
  resource(
    ids.vnetApp,
    "vnet-app",
    "microsoft.network/virtualnetworks",
    "rg-app",
    "sub-prod",
    {
      properties: {
        addressSpace: { addressPrefixes: ["10.1.0.0/16"] },
        subnetCount: 1,
        peeringState: "Connected",
      },
    },
  ),
  resource(
    ids.nsg,
    "nsg-app",
    "microsoft.network/networksecuritygroups",
    "rg-network",
    "sub-prod",
    {
      properties: {
        securityRules: [
          {
            name: "allow-ssh",
            source: "Internet",
            destinationPort: "22",
            access: "Allow",
          },
        ],
      },
    },
  ),
  resource(
    ids.nsgUnused,
    "nsg-unused",
    "microsoft.network/networksecuritygroups",
    "rg-network",
    "sub-prod",
  ),
  resource(
    ids.vm,
    "vm-app-01",
    "microsoft.compute/virtualmachines",
    "rg-app",
    "sub-prod",
    {
      properties: {
        hardwareProfile: { vmSize: "Standard_B2s" },
        provisioningState: "Succeeded",
        zones: ["1"],
      },
      identity: { type: "SystemAssigned" },
    },
  ),
  resource(
    ids.disk,
    "vm-app-01-os",
    "microsoft.compute/disks",
    "rg-app",
    "sub-prod",
    {
      properties: {
        diskSizeGB: 64,
        diskState: "Attached",
        encryption: { type: "EncryptionAtRestWithPlatformKey" },
      },
      sku: { name: "Premium_LRS" },
    },
  ),
  resource(
    ids.nic,
    "vm-app-01-nic",
    "microsoft.network/networkinterfaces",
    "rg-app",
    "sub-prod",
    {
      properties: {
        privateIPAddress: "10.1.0.4",
        enableAcceleratedNetworking: false,
      },
    },
  ),
  resource(
    ids.pip,
    "vm-app-01-pip",
    "microsoft.network/publicipaddresses",
    "rg-app",
    "sub-prod",
    {
      properties: {
        ipAddress: "20.0.0.10",
        publicIPAllocationMethod: "Static",
      },
      sku: { name: "Standard" },
    },
  ),
  resource(
    ids.storage,
    "stprodapp01",
    "microsoft.storage/storageaccounts",
    "rg-app",
    "sub-prod",
    {
      kind: "StorageV2",
      properties: {
        allowBlobPublicAccess: true,
        supportsHttpsTrafficOnly: true,
        minimumTlsVersion: "TLS1_2",
      },
      sku: { name: "Standard_GRS" },
    },
  ),
  resource(
    ids.privateEndpoint,
    "pe-sql",
    "microsoft.network/privateendpoints",
    "rg-app",
    "sub-prod",
    {
      properties: {
        provisioningState: "Succeeded",
        customDnsConfigs: [{ ipAddresses: ["10.1.0.7"] }],
      },
    },
  ),
  resource(ids.sql, "sql-prod", "microsoft.sql/servers", "rg-app", "sub-prod", {
    properties: {
      version: "12.0",
      publicNetworkAccess: "Disabled",
      minimalTlsVersion: "1.2",
    },
  }),
  resource(
    ids.logs,
    "law-prod",
    "microsoft.operationalinsights/workspaces",
    "rg-app",
    "sub-prod",
    {
      properties: {
        retentionInDays: 30,
        publicNetworkAccessForQuery: "Enabled",
        sku: { name: "PerGB2018" },
      },
    },
  ),
  resource(
    ids.hostPool,
    "hp-prod",
    "microsoft.desktopvirtualization/hostpools",
    "rg-app",
    "sub-prod",
    {
      properties: {
        hostPoolType: "Pooled",
        loadBalancerType: "BreadthFirst",
        maxSessionLimit: 10,
      },
    },
  ),
  resource(
    ids.plan,
    "asp-dev",
    "microsoft.web/serverfarms",
    "rg-dev",
    "sub-dev",
    {
      properties: { numberOfSites: 1, reserved: true },
      sku: { name: "B1", tier: "Basic" },
    },
  ),
  resource(ids.web, "web-dev", "microsoft.web/sites", "rg-dev", "sub-dev", {
    kind: "app,linux",
    properties: {
      state: "Running",
      httpsOnly: false,
      defaultHostName: "web-dev.azurewebsites.net",
    },
  }),
];

for (const item of resources) {
  item.findingCount = findings.filter(
    (finding) => finding.resourceId === item.id,
  ).length;
  item.edgeCount = edges.filter(
    (edge) => edge.sourceId === item.id || edge.targetId === item.id,
  ).length;
}

const resourceTypes: ResourceType[] = Object.entries(
  resources.reduce<Record<string, number>>((counts, item) => {
    counts[item.azureType] = (counts[item.azureType] ?? 0) + 1;
    return counts;
  }, {}),
)
  .map(([azureType, count]) => ({
    azureType,
    displayName:
      typeNames[azureType] ?? azureType.split("/").at(-1) ?? azureType,
    count,
    icon: iconData(azureType),
    color: typeColor(azureType),
  }))
  .sort(
    (a, b) => b.count - a.count || a.displayName.localeCompare(b.displayName),
  );

export const mockBootstrap: AppBootstrap = {
  tenants: [
    {
      reference: "contoso",
      name: "Contoso",
      tenantId: "11111111-1111-4111-8111-111111111111",
      configured: true,
    },
    {
      reference: "northwind",
      name: "Northwind Traders",
      tenantId: "44444444-4444-4444-8444-444444444444",
      configured: true,
    },
  ],
  activeTenantId: "11111111-1111-4111-8111-111111111111",
  configError: null,
  labels: DEFAULT_LABELS,
  databasePath: "/Users/demo/Library/Application Support/azdocs/azdocs.db",
  configPath: "/Users/demo/Library/Application Support/azdocs/azdocs.toml",
  configFound: true,
  hasCredentials: true,
  requiredTags: ["env", "owner"],
  latestSnapshotId: "a7f21f53-2026",
  snapshots: [
    {
      id: "a7f21f53-2026",
      createdAt: "2026-08-23T09:42:00Z",
      tenantId: "11111111-1111-4111-8111-111111111111",
      status: "complete",
      notes: "Weekly estate review",
      subscriptions: 2,
      resources: 15,
      findings: 4,
    },
    {
      id: "9d12c813-2026",
      createdAt: "2026-08-16T09:40:00Z",
      tenantId: "11111111-1111-4111-8111-111111111111",
      status: "complete",
      notes: "Before platform release",
      subscriptions: 2,
      resources: 14,
      findings: 5,
    },
    {
      id: "7b42e150-2026",
      createdAt: "2026-08-09T09:39:00Z",
      tenantId: "11111111-1111-4111-8111-111111111111",
      status: "partial",
      notes: "One monitoring query failed",
      subscriptions: 2,
      resources: 14,
      findings: 5,
    },
  ],
};

export const mockQueryPack: QueryDefMeta[] = [
  {
    name: "all_resources",
    category: "inventory",
    kind: "inventory",
    description: "Every resource with identity, tags, SKU and properties.",
  },
  {
    name: "virtual_networks",
    category: "networking",
    kind: "inventory",
    description: "Virtual networks with address space and peering state.",
  },
  {
    name: "storage_accounts",
    category: "storage",
    kind: "inventory",
    description:
      "Storage accounts with TLS floor, HTTPS-only and public access flags.",
  },
  {
    name: "sql_servers_and_dbs",
    category: "databases",
    kind: "inventory",
    description:
      "SQL logical servers with their databases, TLS floor and AAD-only state.",
  },
  {
    name: "storage_public_blob_access",
    category: "storage",
    kind: "finding",
    description: "Storage accounts that allow anonymous public blob access.",
  },
  {
    name: "nsg_open_to_internet",
    category: "networking",
    kind: "finding",
    description: "NSG rules permitting management ports from any source.",
  },
];

export function mockQueryRows(queryName: string): QueryRows {
  if (queryName === "virtual_networks") {
    return {
      queryName,
      columns: [
        "name",
        "resourceGroup",
        "location",
        "addressPrefixes",
        "peeringState",
      ],
      rows: [
        {
          name: "vnet-hub",
          resourceGroup: "rg-network",
          location: "uksouth",
          addressPrefixes: "10.0.0.0/16",
          peeringState: "Connected",
        },
        {
          name: "vnet-app",
          resourceGroup: "rg-app",
          location: "uksouth",
          addressPrefixes: "10.1.0.0/16",
          peeringState: "Connected",
        },
      ],
    };
  }
  if (queryName === "storage_accounts") {
    return {
      queryName,
      columns: [
        "name",
        "resourceGroup",
        "location",
        "minimumTlsVersion",
        "supportsHttpsTrafficOnly",
        "allowBlobPublicAccess",
      ],
      rows: [
        {
          name: "stprodapp01",
          resourceGroup: "rg-app",
          location: "uksouth",
          minimumTlsVersion: "TLS1_2",
          supportsHttpsTrafficOnly: true,
          allowBlobPublicAccess: true,
        },
      ],
    };
  }
  if (queryName === "sql_servers_and_dbs") {
    return {
      queryName,
      columns: [
        "name",
        "resourceGroup",
        "location",
        "minimalTlsVersion",
        "publicNetworkAccess",
      ],
      rows: [
        {
          name: "sql-prod",
          resourceGroup: "rg-app",
          location: "uksouth",
          minimalTlsVersion: "1.2",
          publicNetworkAccess: "Disabled",
        },
      ],
    };
  }
  return {
    queryName,
    columns: ["name", "type", "resourceGroup", "location"],
    rows: resources.map((item) => ({
      name: item.name,
      type: item.azureType,
      resourceGroup: item.resourceGroup,
      location: item.location,
    })),
  };
}

export const mockEstate: EstateSnapshot = {
  id: "a7f21f53-2026",
  createdAt: "2026-08-23T09:42:00Z",
  tenantId: "11111111-1111-4111-8111-111111111111",
  status: "complete",
  notes: "Weekly estate review",
  totals: {
    subscriptions: 2,
    resourceGroups: 3,
    resources: resources.length,
    findings: findings.length,
  },
  tagCoverage: { tagged: 13, untagged: 2, percent: 86 },
  severityCounts: findings.reduce<Record<Severity, number>>(
    (counts, finding) => ({
      ...counts,
      [finding.severity]: counts[finding.severity] + 1,
    }),
    { high: 0, medium: 0, low: 0, info: 0 },
  ),
  // The packaged app gets this from `azdocs::report::governance`, thresholds
  // already applied. Illustrative here, and consistent with the fixture above:
  // everything in Production carries both tags, nothing in Development does.
  governance: {
    distinctKeys: 2,
    topKeys: [
      { key: "env", count: 13, percent: 100 },
      { key: "owner", count: 13, percent: 100 },
    ],
    topKeysTotal: 2,
    subscriptions: [
      {
        subscriptionId: "sub-dev",
        displayName: "Development",
        percent: 0,
        healthy: false,
      },
      {
        subscriptionId: "sub-prod",
        displayName: "Production",
        percent: 100,
        healthy: true,
      },
    ],
    nonCompliant: 2,
    worstGroups: [
      {
        name: "rg-dev",
        subscriptionName: "Development",
        resources: 2,
        nonCompliant: 2,
        missedTags: ["env", "owner"],
        flagged: true,
      },
    ],
    worstGroupsTotal: 1,
  },
  azureMetadata: {
    locations: { uksouth: "UK South", ukwest: "UK West" },
    kinds: {
      "microsoft.documentdb/databaseaccounts:globaldocumentdb":
        "Global Document DB",
      "microsoft.storage/storageaccounts:storagev2": "Storage V2",
      "microsoft.web/sites:app,linux": "App, Linux",
    },
  },
  subscriptions: [
    { id: "sub-prod", displayName: "Production", state: "Enabled" },
    { id: "sub-dev", displayName: "Development", state: "Enabled" },
  ],
  resourceGroups: [
    {
      id: "/subscriptions/sub-prod/resourcegroups/rg-network",
      name: "rg-network",
      subscriptionId: "sub-prod",
      location: "uksouth",
    },
    {
      id: "/subscriptions/sub-prod/resourcegroups/rg-app",
      name: "rg-app",
      subscriptionId: "sub-prod",
      location: "uksouth",
    },
    {
      id: "/subscriptions/sub-dev/resourcegroups/rg-dev",
      name: "rg-dev",
      subscriptionId: "sub-dev",
      location: "ukwest",
    },
  ],
  // The packaged app gets these from Rust. The preview derives them from the
  // same fixture with `buildResourceGroupTopology`, below the declaration —
  // see the note there on why the preview keeps its own copy of that rule.
  resourceGroupSummaries: [],
  resources,
  resourceTypes,
  locations: [
    { name: "uksouth", count: 13 },
    { name: "ukwest", count: 2 },
  ],
  findings,
  edges,
  // Preview-only evidence; production receives these decisions from the Rust report context.
  evidenceSummaries: [
    {
      key: "patch_evaluation_coverage",
      title: "Machine assessment coverage",
      note: "No observed assessment does not establish patch compliance. This browser preview uses fixture data.",
      status: "Recorded evidence",
      columns: ["Type", "State", "Resources"],
      rows: [["Azure virtual machine", "No evaluation observed", "1"]],
    },
  ],
  queryRuns: [
    {
      queryName: "all_resources",
      category: "core",
      rowCount: 15,
      durationMs: 382,
    },
    {
      queryName: "subscriptions",
      category: "core",
      rowCount: 2,
      durationMs: 144,
    },
    {
      queryName: "resource_groups",
      category: "core",
      rowCount: 3,
      durationMs: 164,
    },
    // Representative recorded metadata; other rows exercise older snapshots without it.
    {
      queryName: "virtual_networks",
      category: "networking",
      rowCount: 2,
      durationMs: 277,
      provenance: {
        kql: 'resources\n| where type == "microsoft.network/virtualnetworks"\n| project id, name, location, resourceGroup, subscriptionId,\n          addressPrefixes = properties.addressSpace.addressPrefixes,\n          dnsServers = properties.dhcpOptions.dnsServers,\n          subnetCount = array_length(properties.subnets)\n| order by id asc\n',
        kqlSha256:
          "9d03bc0d75dd5921b322cb424da5c5b5240eade22c09aa1bfee166e82c016e23",
        description: "Virtual networks with address spaces and DNS settings",
        kind: "inventory",
        authorizationScope: "AtScopeAndBelow",
        subscriptions: ["sub-prod", "sub-dev"],
        sourceUrls: [
          "https://learn.microsoft.com/en-us/azure/governance/resource-graph/samples/samples-by-category#virtual-networks",
        ],
        reviewedOn: "2026-09-13",
        revision: null,
      },
    },
    {
      queryName: "storage_public_blob_access",
      category: "storage",
      rowCount: 1,
      durationMs: 219,
    },
    {
      queryName: "nsg_open_to_internet",
      category: "networking",
      rowCount: 1,
      durationMs: 251,
    },
  ],
  previousSnapshotId: "9d12c813-2026",
  previousDiff: {
    baseSnapshotId: "9d12c813-2026",
    targetSnapshotId: "a7f21f53-2026",
    added: [ids.privateEndpoint, ids.sql],
    removed: [
      "/subscriptions/sub-dev/resourcegroups/rg-dev/providers/microsoft.compute/disks/old-test-disk",
    ],
    changed: [ids.web, ids.storage, ids.nsg],
    fields: {
      [ids.web]: [
        { field: "properties", path: "httpsOnly", before: false, after: true },
        { field: "tags", path: "owner", before: null, after: "platform" },
      ],
      [ids.storage]: [
        {
          field: "properties",
          path: "minimumTlsVersion",
          before: "TLS1_0",
          after: "TLS1_2",
        },
      ],
      [ids.nsg]: [
        {
          field: "properties",
          path: "securityRules.0.properties.sourceAddressPrefix",
          before: "*",
          after: "10.0.0.0/8",
        },
      ],
    },
    findingsAdded: [
      {
        queryName: "storage_public_blob_access",
        category: "storage",
        severity: "high",
        resourceId: ids.storage,
        title: "Public blob access enabled",
      },
    ],
    findingsResolved: [
      {
        queryName: "nsg_open_to_internet",
        category: "networking",
        severity: "high",
        resourceId: ids.nsg,
        title: "allow-all: rule permits Internet -> 3389",
      },
      {
        queryName: "missing_required_tags",
        category: "governance",
        severity: "low",
        resourceId: ids.web,
        title: "Missing required tags: owner",
      },
    ],
    edgesAdded: [
      {
        sourceId: ids.privateEndpoint,
        targetId: ids.sql,
        kind: "private_endpoint_for",
      },
    ],
    edgesRemoved: [],
    subscriptionsAdded: [],
    subscriptionsRemoved: [],
    resourceGroupsAdded: [],
    resourceGroupsRemoved: [],
    baseCounts: {
      resources: 14,
      findings: 5,
      high: 2,
      medium: 1,
      low: 2,
      info: 0,
      edges: 11,
      subscriptions: 2,
      resourceGroups: 3,
    },
    targetCounts: {
      resources: 15,
      findings: 4,
      high: 1,
      medium: 1,
      low: 2,
      info: 0,
      edges: 12,
      subscriptions: 2,
      resourceGroups: 3,
    },
  },
  trend: [
    {
      snapshotId: "7b42e150-2026",
      createdAt: "2026-08-09T09:39:00Z",
      status: "partial",
      subscriptions: 2,
      resources: 14,
      tagged: 9,
      findings: 5,
      high: 2,
      medium: 1,
      low: 2,
      info: 0,
      edges: 11,
    },
    {
      snapshotId: "9d12c813-2026",
      createdAt: "2026-08-16T09:40:00Z",
      status: "complete",
      subscriptions: 2,
      resources: 14,
      tagged: 10,
      findings: 5,
      high: 2,
      medium: 1,
      low: 2,
      info: 0,
      edges: 11,
    },
    {
      snapshotId: "a7f21f53-2026",
      createdAt: "2026-08-23T09:42:00Z",
      status: "complete",
      subscriptions: 2,
      resources: 15,
      tagged: 12,
      findings: 4,
      high: 1,
      medium: 1,
      low: 2,
      info: 0,
      edges: 12,
    },
  ],
};

/**
 * Illustrative output paths for the browser preview's Exports workspace.
 *
 * These mirror the real naming in src/commands/report.rs and the desktop's own
 * `diagram_output_target`, and are the third hand-maintained copy of that
 * convention — see the cleanup plan. They exist only so the preview can show a
 * plausible result list; the packaged app returns the paths the Rust exporters
 * actually wrote.
 */
export function mockExportOutputs(request: ExportRequest) {
  const root = request.destination.replace(/[\\/]+$/, "");
  if (request.exportKind === "reports") {
    return request.formats.flatMap((format) => {
      if (format === "md") return [`${root}/docs/index.md`];
      if (format === "html")
        return [`${root}/report.html`, `${root}/docs-html/index.html`];
      if (format === "csv")
        return [`${root}/inventory.csv`, `${root}/findings.csv`];
      if (format === "xlsx") return [`${root}/azdocs.xlsx`];
      return request.includeReference && (format === "pdf" || format === "docx")
        ? [`${root}/report.${format}`, `${root}/technical-reference.${format}`]
        : [`${root}/report.${format}`];
    });
  }

  const diagramType = request.diagramType ?? "network";
  return request.formats.map((format) => {
    const extension = format === "mermaid" ? "mmd" : format;
    if (diagramType === "workbook" && format === "drawio") {
      return `${root}/azdocs-workbook.drawio`;
    }
    if (
      diagramType === "workbook" ||
      diagramType === "vnets" ||
      diagramType === "resource-groups"
    ) {
      return `${root}/diagrams/${diagramType}/example.${extension}`;
    }
    return `${root}/azdocs-${diagramType}.${extension}`;
  });
}

// Filled in after construction because the derivation needs the finished estate.
// `buildResourceGroupTopology` is the browser preview's own implementation of
// the bucketing Rust does in `groups.rs`; it stays here (and only here) because
// the preview has no Rust to call, and it is stripped from the packaged build.
mockEstate.resourceGroupSummaries = buildResourceGroupTopology(
  mockEstate,
).groups.map((group) => ({
  id: group.id,
  name: group.name,
  subscriptionId: group.subscriptionId,
  subscriptionName: group.subscriptionName,
  resourceCount: group.resourceCount,
  findingCount: group.findingCount,
  resourceIds: group.resourceIds,
}));
