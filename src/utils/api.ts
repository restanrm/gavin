import type { Vinyl, User, UploadResponse, BulkImportItem } from '../types';

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
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }

  return response.json();
}

export async function getVinyls(search?: string): Promise<Vinyl[]> {
  const params = search ? `?search=${encodeURIComponent(search)}` : '';
  return fetchJSON<Vinyl[]>(`${BASE_URL}/vinyls${params}`);
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

export async function createVinyl(vinyl: Omit<Vinyl, 'id' | 'created_at'>): Promise<Vinyl> {
  return fetchJSON<Vinyl>(`${BASE_URL}/admin/vinyls`, {
    method: 'POST',
    body: JSON.stringify(vinyl),
  });
}

export async function updateVinyl(id: string, vinyl: Partial<Omit<Vinyl, 'id' | 'created_at'>>): Promise<Vinyl> {
  return fetchJSON<Vinyl>(`${BASE_URL}/admin/vinyls/${id}`, {
    method: 'PUT',
    body: JSON.stringify(vinyl),
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
