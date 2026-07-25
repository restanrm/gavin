import type { Vinyl } from '../types';

interface LibraryStatsProps {
  vinyls: Vinyl[];
  loading?: boolean;
  error?: string | null;
  isAdmin?: boolean;
}

function pluralize(count: number, singular: string, plural = `${singular}s`): string {
  return `${count} ${count === 1 ? singular : plural}`;
}

function needsMetadataReview(status: Vinyl['metadata_status']): boolean {
  return status !== 'complete' && status !== 'disabled';
}

export function LibraryStats({ vinyls, loading = false, error = null, isAdmin = false }: LibraryStatsProps) {
  if (loading) {
    return (
      <div className="library-stats" role="status" aria-live="polite">
        <span className="library-stat">Counting library…</span>
      </div>
    );
  }

  if (error) {
    return null;
  }

  const artists = new Set(
    vinyls
      .map((vinyl) => vinyl.artist.trim().toLocaleLowerCase())
      .filter(Boolean),
  );
  const years = vinyls
    .map((vinyl) => vinyl.release_year)
    .filter((year): year is number => typeof year === 'number')
    .sort((a, b) => a - b);
  const coverCount = vinyls.filter((vinyl) => Boolean(vinyl.cover_image_url)).length;
  const reviewCount = vinyls.filter((vinyl) => needsMetadataReview(vinyl.metadata_status)).length;

  const yearLabel = years.length > 0
    ? years[0] === years[years.length - 1]
      ? `${years[0]}`
      : `${years[0]}–${years[years.length - 1]}`
    : null;

  return (
    <div className="library-stats" aria-label="Library statistics">
      <span className="library-stat">{pluralize(vinyls.length, 'album')}</span>
      <span className="library-stat">{pluralize(artists.size, 'artist')}</span>
      {yearLabel && <span className="library-stat">{yearLabel}</span>}
      <span className="library-stat">{pluralize(coverCount, 'cover')}</span>
      {isAdmin && reviewCount > 0 && (
        <span className="library-stat library-stat-review">
          {pluralize(reviewCount, 'metadata review')}
        </span>
      )}
    </div>
  );
}
