import type { Vinyl, User, UploadResponse, BulkImportItem, AlbumCandidate, CoverImportResponse, AlbumDetails, VinylSort } from '../types';

type VinylInput = Pick<Vinyl, 'artist' | 'title' | 'release_year' | 'genre' | 'notes' | 'cover_image_url'>;
type VinylUpdateInput = Partial<{
  artist: string;
  title: string;
  release_year: number | null;
  genre: string[] | null;
  notes: string | null;
  cover_image_url: string | null;
}>;

export interface MetadataRefreshResponse {
  checked: number;
}

export interface OrphanedImageCleanupResponse {
  deleted: number;
  kept: number;
  errors: string[];
}

const BASE_URL = '/api';

async function fetchJSON<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    let message = `HTTP ${response.status}: ${response.statusText}`;
    try {
      const body = await response.json() as { error?: string };
      if (body.error) {
        message = body.error;
      }
    } catch {
      // Keep the HTTP status message when the response body is not JSON.
    }
    throw new Error(message);
  }

  return response.json();
}

interface VinylListOptions {
  missingMetadataOnly?: boolean;
  genre?: string;
  sort?: VinylSort;
}

export async function getVinyls(search?: string, options: VinylListOptions = {}): Promise<Vinyl[]> {
  const params = new URLSearchParams();
  if (search) {
    params.set('search', search);
  }
  if (options.missingMetadataOnly) {
    params.set('metadata', 'missing');
  }
  if (options.genre) {
    params.set('genre', options.genre);
  }
  if (options.sort && options.sort !== 'artist') {
    params.set('sort', options.sort);
  }

  const query = params.toString();
  return fetchJSON<Vinyl[]>(`${BASE_URL}/vinyls${query ? `?${query}` : ''}`);
}

export async function getVinylDetails(id: string): Promise<AlbumDetails> {
  return fetchJSON<AlbumDetails>(`${BASE_URL}/vinyls/${id}/details`);
}

export async function getAuthStatus(): Promise<User> {
  return fetchJSON<User>(`${BASE_URL}/auth/me`);
}

export function loginRedirect(): void {
  window.location.href = `${BASE_URL}/auth/login`;
}

export async function logout(): Promise<void> {
  await fetch(`${BASE_URL}/auth/logout`, { method: 'POST' });
  window.location.reload();
}

export async function createVinyl(vinyl: VinylInput): Promise<Vinyl> {
  return fetchJSON<Vinyl>(`${BASE_URL}/admin/vinyls`, {
    method: 'POST',
    body: JSON.stringify(vinyl),
  });
}

export async function updateVinyl(id: string, vinyl: VinylUpdateInput): Promise<Vinyl> {
  return fetchJSON<Vinyl>(`${BASE_URL}/admin/vinyls/${id}`, {
    method: 'PUT',
    body: JSON.stringify(vinyl),
  });
}

export async function refreshVinylMetadata(id: string): Promise<Vinyl> {
  return fetchJSON<Vinyl>(`${BASE_URL}/admin/vinyls/${id}/metadata-refresh`, {
    method: 'POST',
  });
}

export async function refreshMissingMetadata(): Promise<MetadataRefreshResponse> {
  return fetchJSON<MetadataRefreshResponse>(`${BASE_URL}/admin/metadata/refresh-missing`, {
    method: 'POST',
  });
}

export async function cleanupOrphanedImages(): Promise<OrphanedImageCleanupResponse> {
  return fetchJSON<OrphanedImageCleanupResponse>(`${BASE_URL}/admin/uploads/cleanup-orphans`, {
    method: 'POST',
  });
}

export async function deleteVinyl(id: string): Promise<void> {
  await fetch(`${BASE_URL}/admin/vinyls/${id}`, {
    method: 'DELETE',
  });
}

export async function bulkImportVinyls(items: BulkImportItem[]): Promise<void> {
  await fetchJSON(`${BASE_URL}/admin/vinyls/bulk`, {
    method: 'POST',
    body: JSON.stringify({ items }),
  });
}

export async function searchArtistAlbums(artist: string): Promise<AlbumCandidate[]> {
  return fetchJSON<AlbumCandidate[]>(
    `${BASE_URL}/admin/albums/search?artist=${encodeURIComponent(artist)}`,
  );
}

export async function createVinylFromCandidate(candidate: AlbumCandidate): Promise<Vinyl> {
  return fetchJSON<Vinyl>(`${BASE_URL}/admin/vinyls/import-cover-candidate`, {
    method: 'POST',
    body: JSON.stringify({ candidate }),
  });
}

export async function selectVinylMetadataCandidate(id: string, candidate: AlbumCandidate): Promise<Vinyl> {
  return fetchJSON<Vinyl>(`${BASE_URL}/admin/vinyls/${id}/metadata-candidate`, {
    method: 'POST',
    body: JSON.stringify({ candidate }),
  });
}

export async function importCoverImage(file: File): Promise<CoverImportResponse> {
  const formData = new FormData();
  formData.append('file', file);

  const response = await fetch(`${BASE_URL}/admin/vinyls/import-cover`, {
    method: 'POST',
    body: formData,
  });

  if (!response.ok) {
    throw new Error(`Cover import failed: ${response.statusText}`);
  }

  return response.json();
}

export async function uploadImage(file: File): Promise<UploadResponse> {
  const formData = new FormData();
  formData.append('file', file);

  const response = await fetch(`${BASE_URL}/admin/uploads`, {
    method: 'POST',
    body: formData,
  });

  if (!response.ok) {
    throw new Error(`Upload failed: ${response.statusText}`);
  }

  return response.json();
}
