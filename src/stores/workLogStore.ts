import { create } from "zustand";
import {
  getDailyWorkAssessment,
  getDailyWorkAssessmentHistory,
  getDailyWorkAssessmentTrend,
  getRhythmProfile,
  getWorkLogReport,
} from "../services/tauriCommands";
import type {
  DailyWorkAssessment,
  DailyWorkAssessmentSummary,
  DailyWorkAssessmentTrend,
} from "../types/dailyWorkAssessment";
import type { RhythmProfile } from "../types/rhythm";
import type { WorkLogReport } from "../types/workLog";

export interface WorkLogStore {
  report: WorkLogReport | null;
  assessment: DailyWorkAssessment | null;
  assessmentHistory: DailyWorkAssessmentSummary[];
  assessmentTrend: DailyWorkAssessmentTrend | null;
  rhythmProfile: RhythmProfile | null;
  selectedDate: string;
  /** Today's report that arrived while viewing another date, to be applied
   *  when the user returns to today (avoids showing stale data). */
  pendingTodayReport: WorkLogReport | null;
  setReport: (report: WorkLogReport) => void;
  setAssessment: (assessment: DailyWorkAssessment) => void;
  setSelectedDate: (date: string) => void;
  loadAssessmentHistory: (limit?: number) => Promise<void>;
  loadAssessmentTrend: (limit?: number) => Promise<void>;
  loadRhythmProfile: () => Promise<void>;
  loadWorkLogReport: (date?: string) => Promise<void>;
}

function todayKey() {
  return new Date().toISOString().slice(0, 10);
}

export const useWorkLogStore = create<WorkLogStore>((set, get) => ({
  report: null,
  assessment: null,
  assessmentHistory: [],
  assessmentTrend: null,
  rhythmProfile: null,
  selectedDate: todayKey(),
  pendingTodayReport: null,
  setReport: (report) => set({ report }),
  setAssessment: (assessment) => set({ assessment }),
  setSelectedDate: (selectedDate) => {
    const previous = get();
    // If the user is switching back to today and a fresher today-report
    // arrived while they were on a historical date, apply it immediately so
    // they don't see stale data (and the caller's load is a no-op refresh).
    // Guard against a midnight rollover: only apply the pending report if it
    // actually corresponds to today (otherwise a report cached before midnight
    // would be wrongly shown for the new day).
    const today = todayKey();
    if (
      selectedDate === today &&
      previous.selectedDate !== today &&
      previous.pendingTodayReport &&
      previous.pendingTodayReport.date === today
    ) {
      set({ selectedDate, report: previous.pendingTodayReport, pendingTodayReport: null });
    } else {
      set({ selectedDate });
    }
  },
  loadAssessmentHistory: async (limit) => {
    try {
      const assessmentHistory = await getDailyWorkAssessmentHistory(limit);
      set({ assessmentHistory });
    } catch (error) {
      console.error("Failed to load assessment history", error);
    }
  },
  loadAssessmentTrend: async (limit) => {
    try {
      const assessmentTrend = await getDailyWorkAssessmentTrend(limit);
      set({ assessmentTrend });
    } catch (error) {
      console.error("Failed to load assessment trend", error);
    }
  },
  loadRhythmProfile: async () => {
    try {
      const rhythmProfile = await getRhythmProfile();
      set({ rhythmProfile });
    } catch (error) {
      console.error("Failed to load rhythm profile", error);
    }
  },
  loadWorkLogReport: async (date) => {
    const selectedDate = date ?? get().selectedDate;
    try {
      const [report, assessment] = await Promise.all([
        getWorkLogReport(selectedDate),
        getDailyWorkAssessment(selectedDate),
      ]);
      set({ assessment, report, selectedDate: report.date });
    } catch (error) {
      console.error("Failed to load work log report", error);
    }
  },
}));
