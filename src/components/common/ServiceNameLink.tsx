import { Typography, Tooltip } from "antd";
import { fontSizes } from "../../styles/tokens";

export interface ServiceNameLinkProps {
  /** 服务名称 */
  name: string;
  /** 服务 ID */
  serviceId: string;
  /** 点击回调 */
  onClick: (serviceId: string) => void;
}

/**
 * 服务名称链接组件 — Typography.Link + Tooltip「点击查看日志」+ hover 下划线。
 * Services / Dashboard 统一使用此组件。
 */
export default function ServiceNameLink({
  name,
  serviceId,
  onClick,
}: ServiceNameLinkProps) {
  return (
    <Tooltip title="点击查看日志">
      <Typography.Link
        onClick={() => onClick(serviceId)}
        style={{ fontSize: fontSizes.Base.size }}
      >
        {name}
      </Typography.Link>
    </Tooltip>
  );
}
