import type { CompanionState } from "../types/companion";

export type KaiVisualState = CompanionState | "searching";
export function KaiAnimatedCharacter({ state }: { state: KaiVisualState }) {
  return <svg className={`kai-svg kai-svg--${state}`} viewBox="0 0 100 100" role="img" aria-label={`Kai is ${state}`}>
    <g className="kai-antenna"><path d="M50 21V11" stroke="#263847" strokeWidth="5" strokeLinecap="round"/><circle cx="50" cy="8" r="6" fill="#55e2ed" stroke="#263847" strokeWidth="3"/><circle className="kai-antenna-light" cx="50" cy="8" r="2.5" fill="#ffcf55"/></g>
    <g className="kai-arms"><path className="kai-arm kai-arm-left" d="M22 60c-10 1-11 12-5 17" fill="none" stroke="#117d95" strokeWidth="12" strokeLinecap="round"/><path className="kai-arm kai-arm-right" d="M78 60c10 1 11 12 5 17" fill="none" stroke="#117d95" strokeWidth="12" strokeLinecap="round"/></g>
    <g className="kai-body"><rect x="18" y="27" width="64" height="57" rx="27" fill="#119bb0" stroke="#263847" strokeWidth="4"/><path d="M24 45c9-16 43-20 54 0v12H24z" fill="#21c5d2" opacity=".58"/>
      <rect x="25" y="39" width="50" height="31" rx="15" fill="#273946" stroke="#17242e" strokeWidth="3"/>
      <rect x="29" y="43" width="42" height="23" rx="11" fill="#ffc84e"/>
      <g className="kai-face"><ellipse className="kai-eye kai-eye-left" cx="42" cy="54" rx="4.3" ry="5.5" fill="#26313c"/><ellipse className="kai-eye kai-eye-right" cx="58" cy="54" rx="4.3" ry="5.5" fill="#26313c"/><g className="kai-pupils"><circle cx="43" cy="53" r="1.4" fill="#fff7cd"/><circle cx="59" cy="53" r="1.4" fill="#fff7cd"/></g><path className="kai-mouth" d="M46 61q4 3 8 0" fill="none" stroke="#75401e" strokeWidth="2.3" strokeLinecap="round"/></g>
      <g className="kai-lids"><path className="kai-lid kai-lid-left" d="M37.5 53q4.5-5 9 0" fill="none" stroke="#ffc84e" strokeWidth="6"/><path className="kai-lid kai-lid-right" d="M53.5 53q4.5-5 9 0" fill="none" stroke="#ffc84e" strokeWidth="6"/></g>
      <circle className="kai-chest-light" cx="50" cy="76" r="5" fill="#ffcf55" stroke="#263847" strokeWidth="2"/>
    </g>
    <g className="kai-sleep"><text x="76" y="29">z</text><text x="84" y="20">z</text></g>
  </svg>;
}
