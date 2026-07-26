import { useState } from 'react';
import { useAuth } from './hooks/useAuth';
import { useVinyls } from './hooks/useVinyls';
import { useDebounce } from './hooks/useDebounce';
import { LoginButton } from './components/LoginButton';
import { ThemeToggle } from './components/ThemeToggle';
import { SearchBar } from './components/SearchBar';
import { VinylCatalog } from './components/VinylCatalog';
import { AdminPanel } from './components/AdminPanel';
import { LibraryStats } from './components/LibraryStats';
import { hasMissingMetadata } from './utils/metadata';
import type { VinylSort } from './types';
import './App.css';

function App() {
  const [searchQuery, setSearchQuery] = useState('');
  const [showMissingMetadataOnly, setShowMissingMetadataOnly] = useState(false);
  const [selectedGenre, setSelectedGenre] = useState('');
  const [sortBy, setSortBy] = useState<VinylSort>('artist');
  const debouncedSearch = useDebounce(searchQuery, 300);
  
  const { isAuthenticated, user, loading: authLoading, refreshAuth } = useAuth();
  const metadataFilterEnabled = isAuthenticated && showMissingMetadataOnly;
  const { vinyls, loading: vinylsLoading, error, refetch } = useVinyls(debouncedSearch, {
    missingMetadataOnly: metadataFilterEnabled,
    genre: selectedGenre,
    sort: sortBy,
  });
  const {
    vinyls: libraryVinyls,
    loading: libraryStatsLoading,
    error: libraryStatsError,
    refetch: refetchLibraryStats,
  } = useVinyls();

  const handleVinylsUpdate = () => {
    refetch();
    refetchLibraryStats();
  };

  const handleLogoutComplete = () => {
    void refreshAuth();
    refetch();
    refetchLibraryStats();
  };

  const searchIsActive = debouncedSearch.trim().length > 0;
  const genreFilterIsActive = selectedGenre.trim().length > 0;
  const catalogFilterIsActive = searchIsActive || metadataFilterEnabled || genreFilterIsActive;
  const missingMetadataCount = vinyls.filter(hasMissingMetadata).length;
  const genres = Array.from(new Set(
    libraryVinyls.flatMap((vinyl) => vinyl.genre ?? []),
  )).sort((left, right) => left.localeCompare(right));
  const emptyCatalogMessage = metadataFilterEnabled
    ? searchIsActive || genreFilterIsActive
      ? 'No albums with missing metadata match your filters.'
      : 'No albums with missing metadata found.'
    : genreFilterIsActive
      ? `No albums found for ${selectedGenre}.`
      : undefined;
  const statsVinyls = catalogFilterIsActive ? libraryVinyls : vinyls;
  const statsLoading = catalogFilterIsActive ? libraryStatsLoading : vinylsLoading;
  const statsError = catalogFilterIsActive ? libraryStatsError : error;

  return (
    <div className="app">
      <header className="app-header">
        <div className="header-content">
          <div className="brand" aria-label="Gavin Vinyl Library">
            <img src="/logo.svg" alt="" className="brand-logo" />
            <h1>Gavin Vinyl Library</h1>
          </div>
          <div className="header-actions">
            <ThemeToggle />
            {!authLoading && (
              <LoginButton
                isAuthenticated={isAuthenticated}
                userName={user?.name || user?.email}
                onLogoutComplete={handleLogoutComplete}
              />
            )}
          </div>
        </div>
      </header>

      <main className="app-main">
        <div className="search-section">
          <SearchBar
            value={searchQuery}
            onChange={setSearchQuery}
            placeholder="Search by artist or title..."
          />

          <div className="catalog-controls" aria-label="Catalog controls">
            <label className="catalog-control">
              <span>Sort by</span>
              <select
                value={sortBy}
                onChange={(event) => setSortBy(event.target.value as VinylSort)}
                className="catalog-select"
              >
                <option value="artist">Artist / title</option>
                <option value="date">Release date</option>
                <option value="last_edit">Last edit</option>
                <option value="genre">Genre</option>
              </select>
            </label>

            <label className="catalog-control">
              <span>Genre</span>
              <select
                value={selectedGenre}
                onChange={(event) => setSelectedGenre(event.target.value)}
                className="catalog-select"
              >
                <option value="">All genres</option>
                {genres.map((genre) => (
                  <option key={genre} value={genre}>{genre}</option>
                ))}
              </select>
            </label>

            {isAuthenticated && (
              <label className="metadata-filter-toggle">
                <input
                  type="checkbox"
                  checked={showMissingMetadataOnly}
                  onChange={(event) => setShowMissingMetadataOnly(event.target.checked)}
                />
                <span>Show only albums with missing metadata</span>
                <span className="metadata-filter-count" aria-label={`${missingMetadataCount} albums with missing metadata`}>
                  {missingMetadataCount}
                </span>
              </label>
            )}
          </div>
        </div>

        {isAuthenticated && (
          <AdminPanel onVinylsUpdate={handleVinylsUpdate} />
        )}

        <section className="catalog-section" aria-label="Vinyl catalog">
          <VinylCatalog
            vinyls={vinyls}
            loading={vinylsLoading}
            error={error}
            isAdmin={isAuthenticated}
            onVinylsUpdate={handleVinylsUpdate}
            emptyMessage={emptyCatalogMessage}
          />
        </section>
      </main>

      <footer className="app-footer">
        <LibraryStats
          vinyls={statsVinyls}
          loading={statsLoading}
          error={statsError}
          isAdmin={isAuthenticated}
        />
        <p>
          Gavin Vinyl Library &copy; {new Date().getFullYear()}
        </p>
      </footer>
    </div>
  );
}

export default App;
