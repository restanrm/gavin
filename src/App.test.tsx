import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import App from './App';
import type { Vinyl } from './types';

const vinyls: Vinyl[] = [
  {
    id: 'complete',
    artist: 'Complete Artist',
    title: 'Complete Record',
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
    genre: ['Rock'],
    release_year: 1970,
    metadata_status: 'complete',
  },
  {
    id: 'missing',
    artist: 'Missing Artist',
    title: 'Missing Metadata Record',
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
    genre: ['Rap'],
    release_year: 1990,
    metadata_status: 'not_found',
  },
];

function jsonResponse(body: unknown) {
  return {
    ok: true,
    json: async () => body,
  } as Response;
}

describe('App admin metadata filter', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('filters the catalog to albums with missing metadata for admins', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn((input: RequestInfo | URL) => {
        const url = input.toString();
        if (url === '/api/auth/me') {
          return Promise.resolve(jsonResponse({ authenticated: true, name: 'Admin' }));
        }
        if (url === '/api/vinyls') {
          return Promise.resolve(jsonResponse(vinyls));
        }
        if (url === '/api/vinyls?metadata=missing') {
          return Promise.resolve(jsonResponse(vinyls.filter((vinyl) => vinyl.metadata_status !== 'complete')));
        }
        return Promise.reject(new Error(`Unexpected fetch: ${url}`));
      }),
    );

    render(<App />);

    expect(await screen.findByText('Complete Record')).toBeInTheDocument();
    expect(screen.getByText('Missing Metadata Record')).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText(/show only albums with missing metadata/i));

    await waitFor(() => {
      expect(screen.queryByText('Complete Record')).not.toBeInTheDocument();
    });
    expect(screen.getByText('Missing Metadata Record')).toBeInTheDocument();
  });

  it('requests genre filtering and catalog sorting', async () => {
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = input.toString();
      if (url === '/api/auth/me') {
        return Promise.resolve(jsonResponse({ authenticated: true, name: 'Admin' }));
      }
      if (url === '/api/vinyls') {
        return Promise.resolve(jsonResponse(vinyls));
      }
      if (url === '/api/vinyls?genre=Rock') {
        return Promise.resolve(jsonResponse(vinyls.filter((vinyl) => vinyl.genre?.includes('Rock'))));
      }
      if (url === '/api/vinyls?genre=Rock&sort=date') {
        return Promise.resolve(jsonResponse(vinyls.filter((vinyl) => vinyl.genre?.includes('Rock'))));
      }
      return Promise.reject(new Error(`Unexpected fetch: ${url}`));
    });
    vi.stubGlobal('fetch', fetchMock);

    render(<App />);

    expect(await screen.findByText('Complete Record')).toBeInTheDocument();

    const controls = within(screen.getByLabelText(/catalog controls/i));

    fireEvent.change(controls.getByLabelText(/^genre$/i), { target: { value: 'Rock' } });
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith('/api/vinyls?genre=Rock', expect.anything()));

    fireEvent.change(controls.getByLabelText(/sort by/i), { target: { value: 'date' } });
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith('/api/vinyls?genre=Rock&sort=date', expect.anything()));
  });
});
