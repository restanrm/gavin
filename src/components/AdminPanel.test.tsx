import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { AdminPanel } from './AdminPanel';

function jsonResponse(body: unknown) {
  return {
    ok: true,
    json: async () => body,
  } as Response;
}

describe('AdminPanel', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    window.localStorage.clear();
  });

  it('persists the collapsed state in local storage', () => {
    const { unmount } = render(<AdminPanel onVinylsUpdate={() => {}} />);

    fireEvent.click(screen.getByRole('button', { name: /hide controls/i }));

    expect(window.localStorage.getItem('gavin-admin-panel-collapsed')).toBe('true');
    expect(screen.queryByText('Maintenance')).not.toBeInTheDocument();

    unmount();
    render(<AdminPanel onVinylsUpdate={() => {}} />);

    expect(screen.getByRole('button', { name: /show controls/i })).toBeInTheDocument();
    expect(screen.queryByText('Maintenance')).not.toBeInTheDocument();
  });

  it('runs metadata refresh and orphaned image cleanup actions', async () => {
    const onVinylsUpdate = vi.fn();
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = input.toString();
      if (url === '/api/admin/metadata/refresh-missing') {
        return Promise.resolve(jsonResponse({ checked: 2 }));
      }
      if (url === '/api/admin/uploads/cleanup-orphans') {
        return Promise.resolve(jsonResponse({ deleted: 3, kept: 4, errors: [] }));
      }
      return Promise.reject(new Error(`Unexpected fetch: ${url}`));
    });
    vi.stubGlobal('fetch', fetchMock);

    render(<AdminPanel onVinylsUpdate={onVinylsUpdate} />);

    fireEvent.click(screen.getByRole('button', { name: /refresh missing metadata/i }));
    expect(await screen.findByText('Metadata refresh checked 2 albums.')).toBeInTheDocument();
    expect(onVinylsUpdate).toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: /clean orphaned images/i }));
    await waitFor(() => {
      expect(screen.getByText('Deleted 3 orphaned images; kept 4 referenced images.')).toBeInTheDocument();
    });

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/admin/metadata/refresh-missing',
      expect.objectContaining({ method: 'POST' }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/admin/uploads/cleanup-orphans',
      expect.objectContaining({ method: 'POST' }),
    );
  });
});
