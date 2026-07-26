import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { LibraryStats } from './LibraryStats';
import type { Vinyl } from '../types';

const vinyls: Vinyl[] = [
  {
    id: '1',
    artist: 'The Beatles',
    title: 'Abbey Road',
    release_year: 1969,
    genre: ['Rock'],
    cover_image_url: 'https://example.com/abbey.jpg',
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
    metadata_status: 'complete',
  },
  {
    id: '2',
    artist: 'Miles Davis',
    title: 'Kind of Blue',
    release_year: 1959,
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
    metadata_status: 'needs_choice',
  },
  {
    id: '3',
    artist: 'the beatles',
    title: 'Revolver',
    release_year: 1966,
    genre: ['Rock'],
    cover_image_url: 'https://example.com/revolver.jpg',
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
    metadata_status: 'complete',
  },
];

describe('LibraryStats', () => {
  it('renders compact library counters', () => {
    render(<LibraryStats vinyls={vinyls} />);

    expect(screen.getByLabelText(/library statistics/i)).toBeInTheDocument();
    expect(screen.getByText('3 albums')).toBeInTheDocument();
    expect(screen.getByText('2 artists')).toBeInTheDocument();
    expect(screen.getByText('1959–1969')).toBeInTheDocument();
    expect(screen.getByText('2 covers')).toBeInTheDocument();
  });

  it('shows metadata review count only for admins', () => {
    const { rerender } = render(<LibraryStats vinyls={vinyls} />);

    expect(screen.queryByText('1 metadata review')).not.toBeInTheDocument();

    rerender(<LibraryStats vinyls={vinyls} isAdmin />);

    expect(screen.getByText('1 metadata review')).toBeInTheDocument();
  });

  it('renders a loading status', () => {
    render(<LibraryStats vinyls={[]} loading />);

    expect(screen.getByRole('status')).toHaveTextContent('Counting library…');
  });
});
