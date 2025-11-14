-- Add workspaces support and persist file contents

-- Workspaces table groups projects per user
CREATE TABLE IF NOT EXISTS workspaces (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Ensure updated_at stays fresh
CREATE TRIGGER update_workspaces_updated_at BEFORE UPDATE ON workspaces
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Link projects to workspaces
ALTER TABLE projects
    ADD COLUMN IF NOT EXISTS workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE;

-- Backfill: create a workspace per owner that currently lacks one
WITH owners_without_workspace AS (
    SELECT DISTINCT p.owner_id
    FROM projects p
    LEFT JOIN workspaces w ON w.owner_id = p.owner_id
    WHERE w.id IS NULL
)
INSERT INTO workspaces (name, description, owner_id)
SELECT
    CONCAT('Imported Workspace ', owner_id::text),
    'Automatically created from existing projects',
    owner_id
FROM owners_without_workspace;

-- Assign each existing project to the earliest workspace owned by the owner
UPDATE projects
SET workspace_id = (
    SELECT w.id
    FROM workspaces w
    WHERE w.owner_id = projects.owner_id
    ORDER BY w.created_at
    LIMIT 1
)
WHERE workspace_id IS NULL;

-- Enforce presence of a workspace link for every project
ALTER TABLE projects
    ALTER COLUMN workspace_id SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_projects_workspace_id ON projects(workspace_id);

-- Persist raw file content directly on the files table
ALTER TABLE files
    ADD COLUMN IF NOT EXISTS content TEXT NOT NULL DEFAULT '';
