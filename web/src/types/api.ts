export interface User {
  id: string;
  username: string;
  admin: boolean;
  enabled: boolean;
}
export interface Session {
  user: User;
  csrf: string;
}
export interface Settings {
  file_limit: number;
  share_limit: number;
  storage_limit: number;
  file_count: number;
  default_days: number | null;
  max_days: number | null;
  max_downloads: number | null;
}
export interface Limits {
  settings: Settings;
  used: number;
}
export interface StoredFile {
  id: string;
  name: string;
  size: number;
  offset: number;
  sha256: string | null;
}
export interface Share {
  id: string;
  title: string;
  state: string;
  files: StoredFile[];
  original_size: number;
  created: number;
  expires: number | null;
  limit: number | null;
  used: number;
  reserved: number;
  password_protected: boolean;
  zip_hash: string | null;
  zip_size: number;
  url: string;
  reason: string | null;
}
export interface PublicShare {
  id: string;
  title: string;
  files: StoredFile[];
  zip_hash: string | null;
  zip_size: number;
  expires: number | null;
  remaining: number | null;
}
export interface Token {
  id: string;
  name: string;
  expires: number;
}
export interface NewShare {
  title: string;
  files: { name: string; size: number }[];
  days: number | null;
  limit: number | null;
  password: string | null;
}
