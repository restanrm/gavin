export interface Vinyl {
  id: string;
  artist: string;
  title: string;
  release_year?: number;
  notes?: string;
  cover_image_url?: string;
  created_at: string;
}

export interface User {
  authenticated: boolean;
  subject?: string;
  email?: string;
  name?: string;
}

export interface UploadResponse {
  url: string;
}

export interface BulkImportItem {
  artist: string;
  title: string;
  year?: number;
  notes?: string;
  cover_url?: string;
}
