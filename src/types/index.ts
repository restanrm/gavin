export interface Vinyl {
  id: string;
  artist: string;
  title: string;
  release_year?: number;
  notes?: string;
  cover_image_url?: string;
  created_at: string;
  metadata_status: 'pending' | 'complete' | 'needs_choice' | 'not_found' | 'error' | 'disabled';
  metadata_source?: string;
  metadata_source_id?: string;
  metadata_source_url?: string;
  metadata_candidates?: string;
  metadata_error?: string;
  metadata_checked_at?: string;
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
  release_year?: number;
  notes?: string;
  cover_url?: string;
  cover_image_url?: string;
}
