import { describe, expect, it } from 'vitest';
import { hasMissingMetadata, missingMetadataItems } from './metadata';
import type { Vinyl } from '../types';

describe('hasMissingMetadata', () => {
  it.each<Vinyl['metadata_status']>(['pending', 'needs_choice', 'not_found', 'error'])(
    'treats %s as missing metadata',
    (metadata_status) => {
      expect(hasMissingMetadata({ metadata_status })).toBe(true);
    },
  );

  it.each<Vinyl['metadata_status']>(['complete', 'disabled'])(
    'does not treat %s as missing metadata',
    (metadata_status) => {
      expect(hasMissingMetadata({ metadata_status })).toBe(false);
    },
  );
});

describe('missingMetadataItems', () => {
  it('explains pending albums with no year or cover', () => {
    expect(
      missingMetadataItems({
        metadata_status: 'pending',
        release_year: null,
        cover_image_url: null,
        metadata_source_url: null,
      }),
    ).toEqual([
      'Metadata lookup has not run yet',
      'Release year',
      'Cover image',
    ]);
  });

  it('explains albums needing a source choice', () => {
    expect(
      missingMetadataItems({
        metadata_status: 'needs_choice',
        release_year: 1969,
        cover_image_url: '/uploads/album-covers/cover.jpg',
        metadata_source_url: null,
      }),
    ).toEqual(['Confirmed metadata source (choose one of the possible matches)']);
  });
});
