import React from 'react';
import { Routes, Route } from 'react-router-dom';
import Container from '@mui/material/Container';
import Layout from './components/Layout';
import Dashboard from './pages/Dashboard';
import Jobs from './pages/Jobs';
import JobDetail from './pages/JobDetail';
import RoutesPage from './pages/Routes';
import NotFound from './pages/NotFound';

function App() {
  return (
    <Layout>
      <Container maxWidth="lg" className="content-wrapper">
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/jobs" element={<Jobs />} />
          <Route path="/jobs/:id" element={<JobDetail />} />
          <Route path="/routes" element={<RoutesPage />} />
          <Route path="*" element={<NotFound />} />
        </Routes>
      </Container>
    </Layout>
  );
}

export default App;
