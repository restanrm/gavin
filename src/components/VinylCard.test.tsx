import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { VinylCard } from '../components/VinylCard';
import type { Vinyl } from '../types';

describe('VinylCard', () => {
  const mockVinyl: Vinyl = {
    id: '1',
    artist: 'The Beatles',
    title: 'Abbey Road',
    release_year: 1969,
    notes: 'Final studio album',
    cover_image_url: 'https://example.com/cover.jpg',
    created_at: '2024-01-01T00:00:00Z',
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

  it('handles optional fields being absent', () => {
    const minimalVinyl: Vinyl = {
      id: '2',
      artist: 'Pink Floyd',
      title: 'The Wall',
      created_at: '2024-01-01T00:00:00Z',
    };

    render(<VinylCard vinyl={minimalVinyl} />);

    expect(screen.getByText('The Wall')).toBeInTheDocument();
    expect(screen.getByText('Pink Floyd')).toBeInTheDocument();
    expect(screen.queryByText(/\d{4}/)).not.toBeInTheDocument(); // No year
  });
});
