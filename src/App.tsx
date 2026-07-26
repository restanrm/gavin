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
import './App.css';

function App() {
  const [searchQuery, setSearchQuery] = useState('');
  const [showMissingMetadataOnly, setShowMissingMetadataOnly] = useState(false);
  const debouncedSearch = useDebounce(searchQuery, 300);
  
  const { isAuthenticated, user, loading: authLoading, refreshAuth } = useAuth();
  const metadataFilterEnabled = isAuthenticated && showMissingMetadataOnly;
  const { vinyls, loading: vinylsLoading, error, refetch } = useVinyls(debouncedSearch, {
    missingMetadataOnly: metadataFilterEnabled,
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
  const catalogFilterIsActive = searchIsActive || metadataFilterEnabled;
  const missingMetadataCount = vinyls.filter(hasMissingMetadata).length;
  const emptyCatalogMessage = metadataFilterEnabled
    ? searchIsActive
      ? 'No albums with missing metadata match your search.'
      : 'No albums with missing metadata found.'
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

          {isAuthenticated && (
            <div className="admin-catalog-filters" aria-label="Admin catalog filters">
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
            </div>
          )}
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
