import { describe, expect, it } from 'vitest';
import { hasMissingMetadata } from './metadata';
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
