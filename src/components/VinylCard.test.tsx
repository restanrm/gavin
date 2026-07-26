import { afterEach, describe, it, expect, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { VinylCard } from '../components/VinylCard';
import type { Vinyl } from '../types';

describe('VinylCard', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  const mockVinyl: Vinyl = {
    id: '1',
    artist: 'The Beatles',
    title: 'Abbey Road',
    release_year: 1969,
    notes: 'Final studio album',
    cover_image_url: 'https://example.com/cover.jpg',
    created_at: '2024-01-01T00:00:00Z',
    metadata_status: 'complete',
  };

  it('renders vinyl information', () => {
    render(<VinylCard vinyl={mockVinyl} />);

    expect(screen.getByText('Abbey Road')).toBeInTheDocument();
    expect(screen.getByText('The Beatles')).toBeInTheDocument();
    expect(screen.getByText('1969')).toBeInTheDocument();
    expect(screen.getByText('Final studio album')).toBeInTheDocument();
  });

  it('renders cover image when provided', () => {
    render(<VinylCard vinyl={mockVinyl} />);

    const img = screen.getByAltText('Abbey Road album cover');
    expect(img).toBeInTheDocument();
    expect(img).toHaveAttribute('src', 'https://example.com/cover.jpg');
  });

  it('hides generated metadata notes from the catalog card', () => {
    render(<VinylCard vinyl={{ ...mockVinyl, notes: 'Metadata: https://musicbrainz.org/release-group/mbid' }} />);

    expect(screen.queryByText(/musicbrainz/)).not.toBeInTheDocument();
  });

  it('opens album details and renders songs when clicked', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        vinyl: mockVinyl,
        release_group_id: 'mbid',
        release_title: 'Abbey Road',
        release_date: '1969-09-26',
        release_country: 'GB',
        release_format: '12" Vinyl',
        source_url: 'https://musicbrainz.org/release/release-id',
        tracklist_status: 'available',
        tracklist_error: null,
        tracks: [
          { disc_number: 1, number: 'A1', title: 'Come Together', length_ms: 259000 },
          { disc_number: 1, number: 'A2', title: 'Something', length_ms: 182000 },
        ],
      }),
    }));

    render(<VinylCard vinyl={mockVinyl} />);

    fireEvent.click(screen.getByRole('button', { name: /view details for abbey road/i }));

    expect(screen.getByRole('dialog', { name: /abbey road/i })).toBeInTheDocument();
    expect(await screen.findByText('Come Together')).toBeInTheDocument();
    expect(screen.getByText('Something')).toBeInTheDocument();
    expect(screen.getByText('1969-09-26')).toBeInTheDocument();
  });

  it('renders placeholder when no cover image', () => {
    const vinylWithoutCover = { ...mockVinyl, cover_image_url: undefined };
    render(<VinylCard vinyl={vinylWithoutCover} />);

    // Placeholder SVG should be present
    const svg = screen.getByRole('article').querySelector('svg');
    expect(svg).toBeInTheDocument();
  });

  it('does not show delete button when not admin', () => {
    render(<VinylCard vinyl={mockVinyl} isAdmin={false} />);

    expect(screen.queryByText('Delete')).not.toBeInTheDocument();
  });

  it('shows delete button when admin', () => {
    render(<VinylCard vinyl={mockVinyl} isAdmin={true} onDelete={() => {}} />);

    expect(screen.getByLabelText(/delete abbey road/i)).toBeInTheDocument();
  });

  it('shows missing metadata details in the album editor', () => {
    const pending: Vinyl = {
      ...mockVinyl,
      release_year: null,
      cover_image_url: null,
      metadata_status: 'pending',
    };

    render(<VinylCard vinyl={pending} isAdmin onUpdate={() => {}} />);

    fireEvent.click(screen.getByLabelText(/edit abbey road/i));

    expect(screen.getByText('Missing or incomplete:')).toBeInTheDocument();
    expect(screen.getByText('Metadata lookup has not run yet')).toBeInTheDocument();
    expect(screen.getByText('Release year')).toBeInTheDocument();
    expect(screen.getByText('Cover image')).toBeInTheDocument();
  });

  it('hides metadata choices from the catalog card until editing', () => {
    const needsChoice: Vinyl = {
      ...mockVinyl,
      metadata_status: 'needs_choice',
      metadata_candidates: JSON.stringify([
        {
          id: 'mbid',
          artist: 'The Beatles',
          title: 'Abbey Road',
          release_year: 1969,
          source_url: 'https://musicbrainz.org/release-group/mbid',
        },
      ]),
    };

    render(<VinylCard vinyl={needsChoice} isAdmin onUpdate={() => {}} />);

    expect(screen.queryByText('Metadata choice required')).not.toBeInTheDocument();
    expect(screen.queryByText('Review possible album matches')).not.toBeInTheDocument();

    fireEvent.click(screen.getByLabelText(/edit abbey road/i));

    expect(screen.getByText('Metadata choice required')).toBeInTheDocument();
    expect(screen.getByText('Review possible album matches')).toBeInTheDocument();
  });

  it('selects a reviewed metadata candidate for an existing vinyl', async () => {
    const onUpdate = vi.fn();
    const candidate = {
      source: 'musicbrainz',
      id: 'mbid',
      artist: 'The Beatles',
      title: 'Abbey Road',
      release_year: 1969,
      source_url: 'https://musicbrainz.org/release-group/mbid',
    };
    const needsChoice: Vinyl = {
      ...mockVinyl,
      metadata_status: 'needs_choice',
      metadata_candidates: JSON.stringify([candidate]),
    };
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ ...needsChoice, metadata_status: 'complete', metadata_source_id: candidate.id }),
    });
    vi.stubGlobal('fetch', fetchMock);

    render(<VinylCard vinyl={needsChoice} isAdmin onUpdate={onUpdate} />);

    fireEvent.click(screen.getByLabelText(/edit abbey road/i));
    fireEvent.click(screen.getByRole('button', { name: /select this match/i }));

    await waitFor(() => expect(onUpdate).toHaveBeenCalled());
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/admin/vinyls/1/metadata-candidate',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ candidate }),
      }),
    );
  });

  it('saves edited information before refreshing album metadata', async () => {
    const onUpdate = vi.fn();
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = input.toString();
      if (url === '/api/admin/vinyls/1') {
        return Promise.resolve({
          ok: true,
          json: async () => ({ ...mockVinyl, title: 'Abbey Road Deluxe' }),
        });
      }
      if (url === '/api/admin/vinyls/1/metadata-refresh') {
        return Promise.resolve({
          ok: true,
          json: async () => ({ ...mockVinyl, title: 'Abbey Road Deluxe', metadata_status: 'complete' }),
        });
      }
      return Promise.reject(new Error(`Unexpected fetch: ${url}`));
    });
    vi.stubGlobal('fetch', fetchMock);

    render(<VinylCard vinyl={mockVinyl} isAdmin onUpdate={onUpdate} />);

    fireEvent.click(screen.getByLabelText(/edit abbey road/i));
    fireEvent.change(screen.getByLabelText(/title/i), {
      target: { value: 'Abbey Road Deluxe' },
    });
    fireEvent.click(screen.getByRole('button', { name: /save & refresh metadata/i }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    expect(onUpdate).not.toHaveBeenCalled();
    expect(screen.getByRole('dialog', { name: /abbey road/i })).toBeInTheDocument();
    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      '/api/admin/vinyls/1',
      expect.objectContaining({
        method: 'PUT',
        body: JSON.stringify({
          artist: 'The Beatles',
          title: 'Abbey Road Deluxe',
          release_year: 1969,
          notes: 'Final studio album',
          cover_image_url: 'https://example.com/cover.jpg',
        }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      '/api/admin/vinyls/1/metadata-refresh',
      expect.objectContaining({ method: 'POST' }),
    );

    fireEvent.click(screen.getByRole('button', { name: /cancel/i }));
    expect(onUpdate).toHaveBeenCalled();
  });

  it('handles optional fields being absent', () => {
    const minimalVinyl: Vinyl = {
      id: '2',
      artist: 'Pink Floyd',
      title: 'The Wall',
      created_at: '2024-01-01T00:00:00Z',
      metadata_status: 'pending',
    };

    render(<VinylCard vinyl={minimalVinyl} />);

    expect(screen.getByText('The Wall')).toBeInTheDocument();
    expect(screen.getByText('Pink Floyd')).toBeInTheDocument();
    expect(screen.queryByText(/\d{4}/)).not.toBeInTheDocument(); // No year
  });
});
