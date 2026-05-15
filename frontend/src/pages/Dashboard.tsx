import React from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery } from 'react-query';
import {
  Box,
  Card,
  CardContent,
  Grid,
  Typography,
  CircularProgress,
  Paper,
  List,
  ListItem,
  ListItemButton,
  ListItemText,
  Chip,
} from '@mui/material';
import { jobsApi, statsApi, Job } from '../api/apiClient';
import PageHeader from '../components/PageHeader';

function StatCard({ label, value, color }: { label: string; value: number; color?: string }) {
  return (
    <Card>
      <CardContent>
        <Typography variant="subtitle2" color="text.secondary">
          {label}
        </Typography>
        <Typography variant="h4" sx={{ color: color || 'text.primary' }}>
          {value}
        </Typography>
      </CardContent>
    </Card>
  );
}

function statusColor(status: Job['status']) {
  switch (status) {
    case 'done':
      return 'success';
    case 'dead':
      return 'error';
    case 'processing':
      return 'info';
    case 'pending':
      return 'warning';
    default:
      return 'default';
  }
}

function Dashboard(): React.ReactElement {
  const navigate = useNavigate();
  const { data: stats, isLoading: statsLoading } = useQuery('stats', statsApi.get, {
    refetchInterval: 5000,
  });
  const { data: recent, isLoading: recentLoading } = useQuery(
    'recent-jobs',
    () => jobsApi.list({ limit: 10 }),
    { refetchInterval: 5000 },
  );

  if (statsLoading || recentLoading) {
    return (
      <Box display="flex" justifyContent="center" mt={4}>
        <CircularProgress />
      </Box>
    );
  }

  const successRate =
    stats && stats.deliveries_total > 0
      ? Math.round((stats.deliveries_success / stats.deliveries_total) * 100)
      : 0;

  return (
    <Box>
      <PageHeader title="Dashboard" subtitle="Operational view of the ingest queue and delivery results" />

      <Grid container spacing={2} sx={{ mb: 3 }}>
        <Grid item xs={12} sm={6} md={3}>
          <StatCard label="Pending" value={stats?.jobs_pending ?? 0} color="#ed6c02" />
        </Grid>
        <Grid item xs={12} sm={6} md={3}>
          <StatCard label="Processing" value={stats?.jobs_processing ?? 0} color="#0288d1" />
        </Grid>
        <Grid item xs={12} sm={6} md={3}>
          <StatCard label="Delivered" value={stats?.jobs_done ?? 0} color="#2e7d32" />
        </Grid>
        <Grid item xs={12} sm={6} md={3}>
          <StatCard label="Dead-lettered" value={stats?.jobs_dead ?? 0} color="#d32f2f" />
        </Grid>
        <Grid item xs={12} sm={6}>
          <StatCard label="Total delivery attempts" value={stats?.deliveries_total ?? 0} />
        </Grid>
        <Grid item xs={12} sm={6}>
          <StatCard label="Delivery success rate (%)" value={successRate} />
        </Grid>
      </Grid>

      <Paper sx={{ p: 2 }}>
        <Typography variant="h6" gutterBottom>
          Recent jobs
        </Typography>
        {recent && recent.length > 0 ? (
          <List dense>
            {recent.map((job) => (
              <ListItem key={job.id} disablePadding>
                <ListItemButton onClick={() => navigate(`/jobs/${job.id}`)}>
                  <ListItemText
                    primary={
                      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                        <Chip label={job.status} size="small" color={statusColor(job.status) as any} />
                        <Typography variant="body2" component="span">
                          {job.route_id}
                        </Typography>
                        <Typography variant="caption" color="text.secondary" component="span">
                          attempts: {job.attempts}
                        </Typography>
                      </Box>
                    }
                    secondary={`${job.id} · ${new Date(job.updated_at).toLocaleString()}`}
                  />
                </ListItemButton>
              </ListItem>
            ))}
          </List>
        ) : (
          <Typography color="text.secondary">No jobs yet.</Typography>
        )}
      </Paper>
    </Box>
  );
}

export default Dashboard;
