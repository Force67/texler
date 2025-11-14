-- Initial schema migration for Texler backend
-- Creates essential tables and types

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create custom types using DO blocks for proper error handling
DO $$ BEGIN
    CREATE TYPE latexengine AS ENUM ('pdflatex', 'xelatex', 'lualatex');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE compilationstatus AS ENUM ('pending', 'running', 'completed', 'failed', 'cancelled');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE outputformat AS ENUM ('pdf', 'dvi', 'ps');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE authmethod AS ENUM ('password', 'oidc');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE collaboratorrole AS ENUM ('viewer', 'editor', 'admin');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255),
    display_name VARCHAR(255),
    avatar_url VARCHAR(500),
    is_active BOOLEAN DEFAULT true,
    email_verified BOOLEAN DEFAULT false,
    auth_method authmethod DEFAULT 'password',
    oidc_provider VARCHAR(50),
    oidc_provider_id VARCHAR(255),
    last_login_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- User preferences table
CREATE TABLE IF NOT EXISTS user_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    theme VARCHAR(20) DEFAULT 'light',
    language VARCHAR(10) DEFAULT 'en',
    latex_engine latexengine DEFAULT 'pdflatex',
    autosave_interval INTEGER DEFAULT 30,
    auto_save INTEGER DEFAULT 30,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Projects table (workspace_id will be added as foreign key in workspaces migration)
CREATE TABLE IF NOT EXISTS projects (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    workspace_id UUID, -- Will be converted to foreign key in workspaces migration
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_public BOOLEAN DEFAULT false,
    main_file_path VARCHAR(500),
    latex_engine latexengine DEFAULT 'pdflatex',
    output_format outputformat DEFAULT 'pdf',
    custom_args TEXT[],
    bibliography_path VARCHAR(500),
    compilation_status compilationstatus DEFAULT 'pending',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Files table
CREATE TABLE IF NOT EXISTS files (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    path VARCHAR(500) NOT NULL,
    content_type VARCHAR(100),
    content TEXT DEFAULT '' NOT NULL,
    storage_strategy VARCHAR(50) DEFAULT 'database',
    content_hash VARCHAR(64),
    size BIGINT DEFAULT 0,
    line_count INTEGER DEFAULT 0,
    word_count INTEGER DEFAULT 0,
    latex_metadata JSONB,
    version INTEGER DEFAULT 1,
    is_main BOOLEAN DEFAULT false,
    is_deleted BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Project collaborators table
CREATE TABLE IF NOT EXISTS project_collaborators (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role collaboratorrole DEFAULT 'viewer',
    permissions JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(project_id, user_id)
);

-- Compilation jobs table
CREATE TABLE IF NOT EXISTS compilation_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    engine latexengine DEFAULT 'pdflatex',
    status compilationstatus DEFAULT 'pending',
    input_files JSONB,
    output_format outputformat DEFAULT 'pdf',
    custom_args TEXT[],
    log_output TEXT,
    artifacts JSONB,
    error_message TEXT,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_projects_owner_id ON projects(owner_id);
CREATE INDEX IF NOT EXISTS idx_projects_workspace_id ON projects(workspace_id);
CREATE INDEX IF NOT EXISTS idx_files_project_id ON files(project_id);
CREATE INDEX IF NOT EXISTS idx_project_collaborators_project_id ON project_collaborators(project_id);
CREATE INDEX IF NOT EXISTS idx_project_collaborators_user_id ON project_collaborators(user_id);
CREATE INDEX IF NOT EXISTS idx_compilation_jobs_project_id ON compilation_jobs(project_id);
CREATE INDEX IF NOT EXISTS idx_compilation_jobs_user_id ON compilation_jobs(user_id);