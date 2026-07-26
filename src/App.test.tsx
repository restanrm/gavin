import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import App from './App';
import type { Vinyl } from './types';

const vinyls: Vinyl[] = [
  {
    id: 'complete',
    artist: 'Complete Artist',
    title: 'Complete Record',
    created_at: '2024-01-01T00:00:00Z',
    metadata_status: 'complete',
  },
  {
    id: 'missing',
    artist: 'Missing Artist',
    title: 'Missing Metadata Record',
    created_at: '2024-01-01T00:00:00Z',
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
});
