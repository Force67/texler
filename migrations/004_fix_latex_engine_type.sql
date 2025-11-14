-- Fix latex_engine column type to use the custom enum instead of VARCHAR
-- This needs to be done carefully due to existing data

-- First, update any existing VARCHAR values to match enum values
UPDATE projects SET latex_engine = 'pdflatex' WHERE latex_engine IS NULL OR latex_engine NOT IN ('pdflatex', 'xelatex', 'lualatex');

-- Drop default constraint temporarily
ALTER TABLE projects ALTER COLUMN latex_engine DROP DEFAULT;

-- Change the column type
ALTER TABLE projects ALTER COLUMN latex_engine TYPE latexengine USING latex_engine::latexengine;

-- Set the default back
ALTER TABLE projects ALTER COLUMN latex_engine SET DEFAULT 'pdflatex'::latexengine;