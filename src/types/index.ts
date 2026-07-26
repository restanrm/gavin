export interface Vinyl {
  id: string;
  artist: string;
  title: string;
  release_year?: number | null;
  genre?: string | null;
  notes?: string | null;
  cover_image_url?: string | null;
  created_at: string;
  metadata_status: 'pending' | 'complete' | 'needs_choice' | 'not_found' | 'error' | 'disabled';
  metadata_source?: string | null;
  metadata_source_id?: string | null;
  metadata_source_url?: string | null;
  metadata_candidates?: string | null;
  metadata_error?: string | null;
  metadata_checked_at?: string | null;
}

export interface AlbumTrack {
  disc_number?: number | null;
  number?: string | null;
  title: string;
  artist?: string | null;
  length_ms?: number | null;
}

export interface AlbumDetails {
  vinyl: Vinyl;
  release_group_id?: string | null;
  release_title?: string | null;
  release_date?: string | null;
  release_country?: string | null;
  release_format?: string | null;
  source_url?: string | null;
  tracklist_status: 'available' | 'not_found' | 'unavailable';
  tracklist_error?: string | null;
  tracks: AlbumTrack[];
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

export interface AlbumCandidate {
  source: string;
  id: string;
  title: string;
  artist: string;
  release_year?: number;
  genre?: string;
  cover_image_url?: string;
  disambiguation?: string;
  source_url: string;
  score?: number;
}

export interface CoverImportResponse {
  status: 'complete' | 'needs_choice' | 'not_found' | 'error';
  detected_terms: string[];
  candidates: AlbumCandidate[];
  vinyl?: Vinyl;
  error?: string;
}

export interface BulkImportItem {
  artist: string;
  title: string;
  year?: number;
  release_year?: number;
  genre?: string;
  notes?: string;
  cover_url?: string;
  cover_image_url?: string;
}
