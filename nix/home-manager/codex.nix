{ agentSkills, ... }:
{
  programs.codex = {
    enable = true;
    skills = agentSkills;
  };
}
