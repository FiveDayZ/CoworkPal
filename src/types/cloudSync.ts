export interface SyncConfig {
  serverUrl: string;
  accessToken: string;
  userName: string;
  tokenRequestId: string;
  tokenRequestSecret: string;
  autoBackupEnabled: boolean;
  autoBackupIntervalMinutes: number;
}

export interface CloudSyncResult {
  revision: string;
  storedAt: string;
  itemCount: number;
}

export interface TokenRequestResult {
  status: "pending" | "approved" | "rejected";
  userName: string;
  requestedAt: string;
  decidedAt: string | null;
  initialSync: CloudSyncResult | null;
  initialSyncError: string | null;
}
