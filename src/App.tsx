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
import './App.css';

function App() {
  const [searchQuery, setSearchQuery] = useState('');
  const debouncedSearch = useDebounce(searchQuery, 300);
  
  const { isAuthenticated, user, loading: authLoading, refreshAuth } = useAuth();
  const { vinyls, loading: vinylsLoading, error, refetch } = useVinyls(debouncedSearch);
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
  const statsVinyls = searchIsActive ? libraryVinyls : vinyls;
  const statsLoading = searchIsActive ? libraryStatsLoading : vinylsLoading;
  const statsError = searchIsActive ? libraryStatsError : error;

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
