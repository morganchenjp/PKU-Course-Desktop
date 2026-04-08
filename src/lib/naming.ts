import type { NamingPatternVars } from "./types";

export function generateFilename(
  pattern: string,
  vars: NamingPatternVars,
  extension: string = 'mp4'
): string {
  let filename = pattern;
  
  // Replace variables
  filename = filename.replace(/{courseName}/g, sanitizeFilename(vars.courseName));
  filename = filename.replace(/{subTitle}/g, sanitizeFilename(vars.subTitle));
  filename = filename.replace(/{lecturerName}/g, sanitizeFilename(vars.lecturerName));
  filename = filename.replace(/{date}/g, vars.date);
  
  if (vars.index !== undefined) {
    filename = filename.replace(/{index}/g, String(vars.index).padStart(2, '0'));
  }
  
  // Add extension
  if (!filename.endsWith(`.${extension}`)) {
    filename += `.${extension}`;
  }
  
  return filename;
}

export function sanitizeFilename(name: string): string {
  // Remove or replace invalid filename characters
  return name
    .replace(/[<>"/\\|?*]/g, '_')
    .replace(/:/g, '：')
    .trim();
}

export function getDefaultNamingPatterns(): { value: string; label: string }[] {
  return [
    { 
      value: '{courseName} - {subTitle} - {lecturerName}', 
      label: '课程名 - 讲次 - 讲师' 
    },
    { 
      value: '{courseName} - {subTitle}', 
      label: '课程名 - 讲次' 
    },
    { 
      value: '{date} - {courseName} - {subTitle}', 
      label: '日期 - 课程名 - 讲次' 
    },
    { 
      value: '{index} - {subTitle} - {lecturerName}', 
      label: '序号 - 讲次 - 讲师' 
    },
  ];
}

export function formatDate(date: Date = new Date()): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}
