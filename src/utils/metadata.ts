import type { Vinyl } from '../types';
import { hasGenres } from './genres';

type MetadataSummaryVinyl = Pick<
  Vinyl,
  'metadata_status' | 'release_year' | 'genre' | 'cover_image_url' | 'metadata_source_url'
>;

export function hasMissingMetadata(
  vinyl: Pick<Vinyl, 'metadata_status'> & Partial<Pick<Vinyl, 'genre'>>,
): boolean {
  return (
    (vinyl.metadata_status !== 'complete' && vinyl.metadata_status !== 'disabled')
    || !hasGenres(vinyl.genre)
  );
}

export function missingMetadataItems(vinyl: MetadataSummaryVinyl): string[] {
  const items: string[] = [];

  switch (vinyl.metadata_status) {
    case 'pending':
      items.push('Metadata lookup has not run yet');
      break;
    case 'needs_choice':
      items.push('Confirmed metadata source (choose one of the possible matches)');
      break;
    case 'not_found':
      items.push('Metadata source match');
      break;
    case 'error':
      items.push('Successful metadata lookup');
      break;
    case 'complete':
    case 'disabled':
      break;
  }

  if (!vinyl.release_year) {
    items.push('Release year');
  }

  if (!hasGenres(vinyl.genre)) {
    items.push('Genre');
  }

  if (!vinyl.cover_image_url) {
    items.push('Cover image');
  }

  if (vinyl.metadata_status === 'complete' && !vinyl.metadata_source_url) {
    items.push('Metadata source link');
  }

  return items;
}
