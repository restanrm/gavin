import { useState, useEffect, useCallback } from 'react';
import { getVinyls } from '../utils/api';
import type { Vinyl } from '../types';

interface UseVinylsOptions {
  missingMetadataOnly?: boolean;
}

export function useVinyls(search?: string, options: UseVinylsOptions = {}) {
  const { missingMetadataOnly = false } = options;
  const [vinyls, setVinyls] = useState<Vinyl[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchVinyls = useCallback(async () => {
    setLoading(true);
    try {
      const data = await getVinyls(search, { missingMetadataOnly });
      setVinyls(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch vinyls');
      setVinyls([]);
    } finally {
      setLoading(false);
    }
  }, [search, missingMetadataOnly]);

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
