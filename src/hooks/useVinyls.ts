import { useState, useEffect, useCallback } from 'react';
import { getVinyls } from '../utils/api';
import type { Vinyl, VinylSort } from '../types';

interface UseVinylsOptions {
  missingMetadataOnly?: boolean;
  genre?: string;
  sort?: VinylSort;
}

export function useVinyls(search?: string, options: UseVinylsOptions = {}) {
  const { missingMetadataOnly = false, genre, sort } = options;
  const [vinyls, setVinyls] = useState<Vinyl[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchVinyls = useCallback(async () => {
    setLoading(true);
    try {
      const data = await getVinyls(search, { missingMetadataOnly, genre, sort });
      setVinyls(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch vinyls');
      setVinyls([]);
    } finally {
      setLoading(false);
    }
  }, [search, missingMetadataOnly, genre, sort]);

  useEffect(() => {
    fetchVinyls();
  }, [fetchVinyls]);

  return {
    vinyls,
    loading,
    error,
    refetch: fetchVinyls,
  };
}
