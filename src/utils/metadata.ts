import type { Vinyl } from '../types';

export function hasMissingMetadata(vinyl: Pick<Vinyl, 'metadata_status'>): boolean {
  return vinyl.metadata_status !== 'complete' && vinyl.metadata_status !== 'disabled';
}
