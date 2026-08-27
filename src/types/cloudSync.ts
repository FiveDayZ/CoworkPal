export interface SyncConfig {
  serverUrl: string;
  accessToken: string;
  userId: string;
  userName: string;
  tokenRequestId: string;
  tokenRequestSecret: string;
  tokenRequestKind: "" | "registration" | "recovery";
  autoBackupEnabled: boolean;
  autoBackupIntervalMinutes: number;
}

export interface CloudSyncResult {
  revision: string;
  storedAt: string;
  itemCount: number;
}

export interface TokenRequestResult {
  kind: "registration" | "recovery";
  status: "pending" | "approved" | "rejected";
  userId: string | null;
  userName: string;
  requestedAt: string;
  decidedAt: string | null;
  initialSync: CloudSyncResult | null;
  initialSyncError: string | null;
}
